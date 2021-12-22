use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::generic_provisioning::{GenericProvisioningPDU, TransactionContinuation, TransactionStart};
use crate::drivers::ble::mesh::provisioning::ProvisioningPDU;
use crate::drivers::ble::mesh::transport::Transport;
use core::marker::PhantomData;
use defmt::Format;
use heapless::Vec;
use crate::drivers::ble::mesh::bearer::advertising::PDU;

pub struct TransactionHandler<T: Transport + 'static> {
    inbound_segments: Option<InboundSegments>,
    outbound_segments: Option<OutboundSegments>,
    _marker: PhantomData<T>,
}

impl<T: Transport + 'static> TransactionHandler<T> {
    pub(crate) fn new() -> Self {
        Self {
            inbound_segments: None,
            outbound_segments: None,
            _marker: PhantomData,
        }
    }

    pub(crate) async fn handle_transaction_start(
        &mut self,
        device: &Device<T>,
        transaction_number: u8,
        transaction_start: &TransactionStart,
    ) -> Result<(), ()> {
        if transaction_start.seg_n == 0 {
            let pdu = ProvisioningPDU::parse(&*transaction_start.data)?;
            device.handle_provisioning_pdu(pdu).await?;
            device.tx_transaction_ack(transaction_number).await?;
        } else {
            self.inbound_segments = Some(InboundSegments::new(
                transaction_number,
                transaction_start.seg_n,
                &transaction_start.data,
            ));
        }

        Ok(())
    }

    pub(crate) async fn handle_transaction_continuation(
        &mut self,
        device: &Device<T>,
        transaction_number: u8,
        transaction_continuation: &TransactionContinuation,
    ) -> Result<(), ()> {
        if self.inbound_segments.as_mut().ok_or(())?.receive(
            transaction_number,
            transaction_continuation.segment_index,
            &transaction_continuation.data,
        )? {
            let mut data: Vec<u8, 1024> = Vec::new();
            self.inbound_segments.as_ref().ok_or(())?.fill(&mut data);
            let pdu = ProvisioningPDU::parse(&*data)?;
            device.handle_provisioning_pdu(pdu).await?;
            device.tx_transaction_ack(transaction_number).await?;
        }

        Ok(())
    }

    pub(crate) async fn handle_outbound(&mut self, device: &Device<T>, pdu: Option<ProvisioningPDU>) -> Result<(), ()> {
        defmt::info!("&& A");
        match pdu {
            None => {
                defmt::info!("&& B");
            }
            Some(pdu) => {
                defmt::info!("&& C");
                if self.outbound_segments.is_some() {
                    defmt::info!("&& D {}", self.outbound_segments);
                    //return Err(());
                }
                defmt::info!("&& E");
                self.outbound_segments.replace(OutboundSegments::new(device.next_transaction(), pdu) );
            }
        }
        defmt::info!("&& F");

        if let Some(segments) = &self.outbound_segments {
            defmt::info!("TRANSMIT OUTBOUND TRANSACTION");
            segments.transmit(device).await?;
        }
        defmt::info!("&& G");

        Ok(())
    }

    pub(crate) async fn handle_transaction_ack() {
        // ignorable for this role?
    }
}

struct InboundSegments {
    transaction_number: u8,
    segments: Vec<Option<Vec<u8, 64>>, 32>,
}

impl InboundSegments {
    fn new(transaction_number: u8, seg_n: u8, data: &Vec<u8, 64>) -> Self {
        let mut this = Self {
            transaction_number,
            segments: Vec::new(),
        };
        for n in 0..seg_n + 1 {
            this.segments.push(None);
        }
        let mut chunk = Vec::new();
        chunk.extend_from_slice(data);
        this.segments[0] = Some(chunk);
        this
    }

    fn is_complete(&self) -> bool {
        self.segments.iter().all(|e| matches!(e, Some(_)))
    }

    pub(crate) fn receive(
        &mut self,
        transaction_number: u8,
        segment_index: u8,
        data: &Vec<u8, 64>,
    ) -> Result<bool, ()> {
        if transaction_number != self.transaction_number {
            return Err(());
        }

        if let None = self.segments[segment_index as usize] {
            let mut chunk = Vec::new();
            chunk.extend_from_slice(data);
            self.segments[segment_index as usize] = Some(chunk);
        }

        Ok(self.is_complete())
    }

    pub(crate) fn fill<const N: usize>(&self, dst: &mut Vec<u8, N>) -> Result<(), ()> {
        for chunk in &self.segments {
            dst.extend_from_slice(&chunk.as_ref().ok_or(())?)?
        }

        Ok(())
    }
}

#[derive(Format)]
struct OutboundSegments {
    transaction_number: u8,
    pdu: Vec<u8, 256>,
    orig: ProvisioningPDU,
}

impl OutboundSegments {
    pub fn new(transaction_number: u8, pdu: ProvisioningPDU) -> Self {
        let mut data = Vec::new();
        pdu.emit( &mut data);
        Self {
            transaction_number,
            pdu: data,
            orig: pdu,
        }
    }

    pub async fn transmit<T: Transport + 'static>(&self, device: &Device<T>) -> Result<(), ()> {
        let num_chunks = self.pdu.chunks(15).count();
        defmt::info!("chunks {}", num_chunks);
        for (i, chunk) in self.pdu.chunks(15).enumerate() {
            let pdu = if i == 0 {
                GenericProvisioningPDU::TransactionStart(TransactionStart {
                    seg_n: num_chunks as u8 - 1,
                    total_len: self.pdu.len() as u16,
                    fcs: 0,
                    data: Vec::from_slice(chunk)?,
                })
            } else {
                GenericProvisioningPDU::TransactionContinuation(TransactionContinuation {
                    segment_index: i as u8 + 1,
                    data: Vec::from_slice(chunk)?
                })
            };

            let pdu = PDU {
                link_id: device.link_id()?,
                transaction_number: self.transaction_number,
                pdu,
            };

            defmt::info!("tx via btle");
            device.tx_pdu(pdu).await;
            defmt::info!("tx'd via btle");
        }
        Ok(())
    }
}
