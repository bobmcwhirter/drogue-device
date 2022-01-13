use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::provisioning::Provisioning;
use crate::actors::ble::mesh::provisioning_bearer::{ProvisioningBearer, ProvisioningBearerMessage};
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::generic_provisioning::GenericProvisioningPDU;
use crate::drivers::ble::mesh::provisioning::ProvisioningPDU;
use crate::drivers::ble::mesh::transport::Transport;
use crate::drivers::ble::mesh::InsufficientBuffer;
use crate::{Actor, Address, Inbox};
use core::future::Future;
use heapless::Vec;

pub struct Transaction<T>
where
    T: Transport + 'static,
{
    inbound_segments: Option<InboundSegments>,
    down: Option<Address<ProvisioningBearer<T>>>,
    up: Option<Address<Provisioning>>,
    inbound_acks: Option<u8>,
}

impl<T> Transaction<T>
where
    T: Transport + 'static,
{
    pub fn new() -> Self {
        Self {
            inbound_segments: None,
            down: None,
            up: None,
            inbound_acks: None,
        }
    }

    fn already_acked(&self, transaction_number: u8) -> bool {
        if let Some(ack) = self.inbound_acks {
            return transaction_number <= ack;
        } else {
            false
        }
    }

    async fn check_ack(
        &mut self,
        transaction_number: u8,
    ) -> Result<bool, DeviceError> {
        if !self.already_acked(transaction_number) {
            Ok(false)
        } else {
            self.do_ack(transaction_number).await?;
            Ok(true)
        }
    }

    async fn do_ack(
        &mut self,
        transaction_number: u8,
    ) -> Result<(), DeviceError> {
        let ack = GenericProvisioningPDU::TransactionAck;
        self.down.unwrap().notify( ProvisioningBearerMessage::Outbound( transaction_number, ack ) );
        match self.inbound_acks{
            None => {
                self.inbound_acks.replace(transaction_number);
            }
            Some(latest) => {
                // TODO fix wraparound rollover.
                if transaction_number > latest {
                    self.inbound_acks.replace(transaction_number);
                }
            }
        }
        Ok(())
    }
}

pub enum TransactionMessage<T>
where
    T: Transport + 'static,
{
    Initialize(Address<ProvisioningBearer<T>>, Address<Provisioning>),
    Inbound(PDU),
}

impl<T> Actor for Transaction<T>
where
    T: Transport + 'static,
{
    type Message<'m> = TransactionMessage<T>;
    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl Future<Output = ()> + 'm;

    fn on_mount<'m, M>(
        &'m mut self,
        _: Address<Self>,
        inbox: &'m mut M,
    ) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {
            loop {
                let mut message = inbox.next().await;
                if let Some(mut message) = message {
                    match message.message() {
                        TransactionMessage::Initialize(down, up) => {
                            self.down.replace(*down);
                            self.up.replace(*up);
                        }
                        TransactionMessage::Inbound(pdu) => {
                            match &pdu.pdu {
                                GenericProvisioningPDU::TransactionStart(transaction_start) => {
                                    if transaction_start.seg_n == 0 {
                                        if let Ok(pdu) =
                                            ProvisioningPDU::parse(&*transaction_start.data)
                                        {
                                            self.up.unwrap().request(pdu).unwrap().await;
                                        }
                                        //self.do_ack(device, transaction_number).await?;
                                        //device.handle_provisioning_pdu(pdu).await?;
                                    } else {
                                        let needs_new = match &self.inbound_segments {
                                            Some(segments)
                                                if pdu.transaction_number
                                                    == segments.transaction_number =>
                                            {
                                                false
                                            }
                                            _ => true,
                                        };

                                        match &self.inbound_segments {
                                            Some(segments)
                                                if pdu.transaction_number
                                                    == segments.transaction_number =>
                                            {
                                                // nothing
                                            }
                                            _ => {
                                                if let Ok(segments) = InboundSegments::new(
                                                    pdu.transaction_number,
                                                    transaction_start.seg_n,
                                                    &transaction_start.data,
                                                ) {
                                                    self.inbound_segments.replace(segments);
                                                }
                                            }
                                        }
                                    }
                                }
                                GenericProvisioningPDU::TransactionContinuation(
                                    transaction_continuation,
                                ) => {
                                    if let Some(segments) = &mut self.inbound_segments {
                                        if let Ok(Some(provisioning_pdu)) = segments.receive(
                                            pdu.transaction_number,
                                            transaction_continuation.segment_index,
                                            &transaction_continuation.data,
                                        ) {
                                            self.up.unwrap().request(provisioning_pdu).unwrap().await;
                                        }
                                    }
                                }
                                _ => {
                                    // ignore
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

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
    ) -> Result<Option<ProvisioningPDU>, DeviceError> {
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

        if self.is_complete() {
            let mut data: Vec<u8, 1024> = Vec::new();
            self.fill(&mut data)?;
            let pdu = ProvisioningPDU::parse(&*data)?;
            Ok(Some(pdu))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn fill<const N: usize>(&self, dst: &mut Vec<u8, N>) -> Result<(), DeviceError> {
        for chunk in &self.segments {
            dst.extend_from_slice(&chunk.as_ref().ok_or(DeviceError::IncompleteTransaction)?)
                .map_err(|_| DeviceError::InsufficientBuffer)?
        }

        Ok(())
    }
}
