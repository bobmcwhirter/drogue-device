use crate::actors::ble::mesh::device::{Device, DeviceError};
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::generic_provisioning::{
    GenericProvisioningPDU, TransactionContinuation, TransactionStart,
};
use crate::drivers::ble::mesh::provisioning::ProvisioningPDU;
use crate::drivers::ble::mesh::transport::Transport;
use crate::drivers::ble::mesh::InsufficientBuffer;
use core::cell::RefCell;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};
use defmt::Format;
use heapless::Vec;
use rand_core::{CryptoRng, RngCore};
use crate::drivers::ble::mesh::key_storage::KeyStorage;

pub struct TransactionHandler<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: KeyStorage + 'static,
{
    inbound_segments: RefCell<Option<InboundSegments>>,
    inbound_acks: RefCell<Option<u8>>,
    outbound_segments: Option<OutboundSegments>,
    _marker: PhantomData<(T, R, S)>,
}

impl<T, R, S> TransactionHandler<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: KeyStorage + 'static,
{
    pub(crate) fn new() -> Self {
        Self {
            inbound_segments: RefCell::new(None),
            inbound_acks: RefCell::new(None),
            outbound_segments: None,
            _marker: PhantomData,
        }
    }

    pub(crate) fn link_close(&mut self) {
        self.inbound_segments.borrow_mut().take();
        self.inbound_acks.borrow_mut().take();
        self.outbound_segments.take();
    }

    fn already_acked(&self, transaction_number: u8) -> bool {
        if let Some(ack) = *self.inbound_acks.borrow() {
            return transaction_number <= ack;
        } else {
            false
        }
    }

    async fn check_ack(
        &mut self,
        device: &Device<T, R, S>,
        transaction_number: u8,
    ) -> Result<bool, DeviceError> {
        if !self.already_acked(transaction_number) {
            Ok(false)
        } else {
            self.do_ack(device, transaction_number).await?;
            Ok(true)
        }
    }

    async fn do_ack(
        &self,
        device: &Device<T, R, S>,
        transaction_number: u8,
    ) -> Result<(), DeviceError> {
        device.tx_transaction_ack(transaction_number).await?;
        let mut acks = self.inbound_acks.borrow_mut();
        match *acks {
            None => {
                acks.replace(transaction_number);
            }
            Some(latest) => {
                // TODO fix wraparound rollover.
                if transaction_number > latest {
                    acks.replace(transaction_number);
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn handle_transaction_start(
        &mut self,
        device: &Device<T, R, S>,
        transaction_number: u8,
        transaction_start: &TransactionStart,
    ) -> Result<(), DeviceError> {
        if self.check_ack(device, transaction_number).await? {
            return Ok(());
        }

        if transaction_start.seg_n == 0 {
            let pdu = ProvisioningPDU::parse(&*transaction_start.data)?;
            self.do_ack(device, transaction_number).await?;
            device.handle_provisioning_pdu(pdu).await?;
        } else {
            let needs_new = match &*self.inbound_segments.borrow() {
                Some(inbound) if inbound.transaction_number == transaction_number => false,
                _ => true,
            };
            if needs_new {
                self.inbound_segments
                    .borrow_mut()
                    .replace(InboundSegments::new(
                        transaction_number,
                        transaction_start.seg_n,
                        &transaction_start.data,
                    )?);
            }
        }

        Ok(())
    }

    pub(crate) async fn handle_transaction_continuation(
        &mut self,
        device: &Device<T, R, S>,
        transaction_number: u8,
        transaction_continuation: &TransactionContinuation,
    ) -> Result<(), DeviceError> {
        if self.check_ack(device, transaction_number).await? {
            return Ok(());
        }

        let mut complete = false;

        let result = if let Some(inbound) = self.inbound_segments.borrow_mut().as_mut() {
            if inbound.receive(
                transaction_number,
                transaction_continuation.segment_index,
                &transaction_continuation.data,
            )? {
                complete = true;
                self.do_ack(device, transaction_number).await?;

                let mut data: Vec<u8, 1024> = Vec::new();
                inbound.fill(&mut data)?;
                let pdu = ProvisioningPDU::parse(&*data)?;
                device.handle_provisioning_pdu(pdu).await
            } else {
                // inbound segments are not yet complete, let it come around again.
                Ok(())
            }
        } else {
            // Maybe we just missed the first TransactionStart, so not an error,
            // hope a retransmit finds its way to us.
            Ok(())
        };

        if complete {
            self.inbound_segments.borrow_mut().take();
        }

        result
    }

    pub(crate) async fn handle_outbound(
        &mut self,
        device: &Device<T, R, S>,
        pdu: Option<ProvisioningPDU>,
    ) -> Result<(), DeviceError> {
        match pdu {
            None => {
                // nothing
            }
            Some(pdu) => {
                if self.outbound_segments.is_some() {
                    // TODO check transaction_number
                    //return Err(());
                }
                self.outbound_segments
                    .replace(OutboundSegments::new(device.next_transaction(), pdu));
            }
        }

        if let Some(segments) = &self.outbound_segments {
            segments.transmit(device).await?;
        }

        Ok(())
    }

    pub(crate) async fn handle_transaction_ack(
        &mut self,
        device: &Device<T, R, S>,
        transaction_number: u8,
    ) -> Result<(), DeviceError> {
        match &self.outbound_segments {
            None => { /* nothing */ }
            Some(segments) => {
                if segments.transaction_number == transaction_number {
                    defmt::trace!(">> TransactionAck({})", segments.transaction_number);
                    self.outbound_segments.take();
                }
            }
        }

        Ok(())
    }
}

struct IncompleteTransaction;

struct InboundSegments {
    transaction_number: u8,
    segments: Vec<Option<Vec<u8, 64>>, 32>,
}

impl InboundSegments {
    fn new(
        transaction_number: u8,
        seg_n: u8,
        data: &Vec<u8, 64>,
    ) -> Result<Self, InsufficientBuffer> {
        let mut this = Self {
            transaction_number,
            segments: Vec::new(),
        };
        for _ in 0..seg_n + 1 {
            this.segments.push(None).map_err(|_| InsufficientBuffer)?;
        }
        let mut chunk = Vec::new();
        chunk
            .extend_from_slice(data)
            .map_err(|_| InsufficientBuffer)?;
        this.segments[0] = Some(chunk);
        Ok(this)
    }

    fn is_complete(&self) -> bool {
        self.segments.iter().all(|e| matches!(e, Some(_)))
    }

    pub(crate) fn receive(
        &mut self,
        transaction_number: u8,
        segment_index: u8,
        data: &Vec<u8, 64>,
    ) -> Result<bool, DeviceError> {
        if transaction_number != self.transaction_number {
            return Err(DeviceError::InvalidTransactionNumber);
        }

        if let None = self.segments[segment_index as usize] {
            let mut chunk = Vec::new();
            chunk
                .extend_from_slice(data)
                .map_err(|_| DeviceError::InsufficientBuffer)?;
            self.segments[segment_index as usize] = Some(chunk);
        }

        Ok(self.is_complete())
    }

    pub(crate) fn fill<const N: usize>(&self, dst: &mut Vec<u8, N>) -> Result<(), DeviceError> {
        for chunk in &self.segments {
            dst.extend_from_slice(&chunk.as_ref().ok_or(DeviceError::IncompleteTransaction)?)
                .map_err(|_| DeviceError::InsufficientBuffer)?
        }

        Ok(())
    }
}

struct OutboundSegments {
    transaction_number: u8,
    pdu: Vec<u8, 128>,
    num_segments: u8,
    fcs: u8,
    first: AtomicBool,
}

const TRANSACTION_START_MTU: usize = 20;
const TRANSACTION_CONTINUATION_MTU: usize = 23;

impl OutboundSegments {
    pub fn new(transaction_number: u8, pdu: ProvisioningPDU) -> Self {
        let mut data = Vec::new();
        pdu.emit(&mut data);
        let fcs = fcs(&data);
        let num_segments = Self::num_chunks(&data);

        Self {
            transaction_number,
            pdu: data,
            num_segments,
            fcs: fcs,
            first: AtomicBool::new(true),
        }
    }

    pub async fn transmit<T, R, S>(&self, device: &Device<T, R, S>) -> Result<(), DeviceError>
    where
        T: Transport + 'static,
        R: RngCore + CryptoRng + 'static,
        S: KeyStorage + 'static,
    {
        if self.first.swap(false, Ordering::SeqCst) {
            defmt::trace!("<< Transaction({})", self.transaction_number);
        }

        let iter = OutboundSegmentsIter::new(self);

        if let Some(link_id) = device.link_id() {
            for pdu in iter {
                device
                    .tx_pdu(PDU {
                        link_id,
                        transaction_number: self.transaction_number,
                        pdu,
                    })
                    .await?
            }
            Ok(())
        } else {
            Err(DeviceError::NoEstablishedLink)
        }
    }

    fn num_chunks(pdu: &[u8]) -> u8 {
        let mut len = pdu.len();
        // TransactionStart can hold 20
        if len <= TRANSACTION_START_MTU {
            return 1;
        }
        let mut num_chunks = 1;
        len = len - TRANSACTION_START_MTU;
        // TransactionContinuation can hold 24
        while len > 0 {
            num_chunks = num_chunks + 1;
            if len > TRANSACTION_CONTINUATION_MTU {
                len = len - TRANSACTION_CONTINUATION_MTU;
            } else {
                break;
            }
        }

        num_chunks
    }
}

struct OutboundSegmentsIter<'a> {
    segments: &'a OutboundSegments,
    cur: usize,
}

impl<'a> OutboundSegmentsIter<'a> {
    fn new(segments: &'a OutboundSegments) -> Self {
        Self { segments, cur: 0 }
    }
}

impl<'a> Iterator for OutboundSegmentsIter<'a> {
    type Item = GenericProvisioningPDU;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.cur;
        self.cur = self.cur + 1;

        if cur == 0 {
            let chunk = if self.segments.pdu.len() <= TRANSACTION_START_MTU {
                &self.segments.pdu
            } else {
                &self.segments.pdu[0..TRANSACTION_START_MTU]
            };

            Some(GenericProvisioningPDU::TransactionStart(TransactionStart {
                seg_n: self.segments.num_segments as u8 - 1,
                total_len: self.segments.pdu.len() as u16,
                fcs: self.segments.fcs,
                data: Vec::from_slice(chunk).ok()?,
            }))
        } else {
            let chunk_start = TRANSACTION_START_MTU + ((cur - 1) * TRANSACTION_CONTINUATION_MTU);
            if chunk_start >= self.segments.pdu.len() {
                None
            } else {
                let chunk_end = chunk_start + TRANSACTION_CONTINUATION_MTU;
                let chunk = if chunk_end <= self.segments.pdu.len() {
                    &self.segments.pdu[chunk_start..chunk_end]
                } else {
                    &self.segments.pdu[chunk_start..]
                };
                Some(GenericProvisioningPDU::TransactionContinuation(
                    TransactionContinuation {
                        segment_index: cur as u8,
                        data: Vec::from_slice(chunk).ok()?,
                    },
                ))
            }
        }
    }
}

const CRCTABLE: [u8; 256] = [
    0x00, 0x91, 0xE3, 0x72, 0x07, 0x96, 0xE4, 0x75, 0x0E, 0x9F, 0xED, 0x7C, 0x09, 0x98, 0xEA, 0x7B,
    0x1C, 0x8D, 0xFF, 0x6E, 0x1B, 0x8A, 0xF8, 0x69, 0x12, 0x83, 0xF1, 0x60, 0x15, 0x84, 0xF6, 0x67,
    0x38, 0xA9, 0xDB, 0x4A, 0x3F, 0xAE, 0xDC, 0x4D, 0x36, 0xA7, 0xD5, 0x44, 0x31, 0xA0, 0xD2, 0x43,
    0x24, 0xB5, 0xC7, 0x56, 0x23, 0xB2, 0xC0, 0x51, 0x2A, 0xBB, 0xC9, 0x58, 0x2D, 0xBC, 0xCE, 0x5F,
    0x70, 0xE1, 0x93, 0x02, 0x77, 0xE6, 0x94, 0x05, 0x7E, 0xEF, 0x9D, 0x0C, 0x79, 0xE8, 0x9A, 0x0B,
    0x6C, 0xFD, 0x8F, 0x1E, 0x6B, 0xFA, 0x88, 0x19, 0x62, 0xF3, 0x81, 0x10, 0x65, 0xF4, 0x86, 0x17,
    0x48, 0xD9, 0xAB, 0x3A, 0x4F, 0xDE, 0xAC, 0x3D, 0x46, 0xD7, 0xA5, 0x34, 0x41, 0xD0, 0xA2, 0x33,
    0x54, 0xC5, 0xB7, 0x26, 0x53, 0xC2, 0xB0, 0x21, 0x5A, 0xCB, 0xB9, 0x28, 0x5D, 0xCC, 0xBE, 0x2F,
    0xE0, 0x71, 0x03, 0x92, 0xE7, 0x76, 0x04, 0x95, 0xEE, 0x7F, 0x0D, 0x9C, 0xE9, 0x78, 0x0A, 0x9B,
    0xFC, 0x6D, 0x1F, 0x8E, 0xFB, 0x6A, 0x18, 0x89, 0xF2, 0x63, 0x11, 0x80, 0xF5, 0x64, 0x16, 0x87,
    0xD8, 0x49, 0x3B, 0xAA, 0xDF, 0x4E, 0x3C, 0xAD, 0xD6, 0x47, 0x35, 0xA4, 0xD1, 0x40, 0x32, 0xA3,
    0xC4, 0x55, 0x27, 0xB6, 0xC3, 0x52, 0x20, 0xB1, 0xCA, 0x5B, 0x29, 0xB8, 0xCD, 0x5C, 0x2E, 0xBF,
    0x90, 0x01, 0x73, 0xE2, 0x97, 0x06, 0x74, 0xE5, 0x9E, 0x0F, 0x7D, 0xEC, 0x99, 0x08, 0x7A, 0xEB,
    0x8C, 0x1D, 0x6F, 0xFE, 0x8B, 0x1A, 0x68, 0xF9, 0x82, 0x13, 0x61, 0xF0, 0x85, 0x14, 0x66, 0xF7,
    0xA8, 0x39, 0x4B, 0xDA, 0xAF, 0x3E, 0x4C, 0xDD, 0xA6, 0x37, 0x45, 0xD4, 0xA1, 0x30, 0x42, 0xD3,
    0xB4, 0x25, 0x57, 0xC6, 0xB3, 0x22, 0x50, 0xC1, 0xBA, 0x2B, 0x59, 0xC8, 0xBD, 0x2C, 0x5E, 0xCF,
];

fn fcs(data: &[u8]) -> u8 {
    let mut fcs = 0xff;

    for b in data {
        fcs = CRCTABLE[(fcs ^ b) as usize];
    }
    0xFF - fcs
}
