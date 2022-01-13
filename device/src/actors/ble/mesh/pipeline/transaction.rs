use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::provisioning_bearer::ProvisioningBearerContext;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::generic_provisioning::GenericProvisioningPDU;
use crate::drivers::ble::mesh::provisioning::ProvisioningPDU;
use core::future::Future;
use heapless::Vec;
use crate::drivers::ble::mesh::InsufficientBuffer;

pub trait TransactionContext: ProvisioningBearerContext {
    type TransactionTransmitFuture<'m>: Future<Output = Result<(), DeviceError>>
    where
        Self: 'm;

    fn transmit_transaction_ack<'m>(&mut self) -> Self::TransactionTransmitFuture<'m>;
}

pub struct Transaction {
    inbound_segments: Option<InboundSegments>,
    inbound_acks: Option<u8>,
}

impl Transaction {
    pub async fn process<C: TransactionContext>(
        &mut self,
        ctx: &mut C,
        pdu: GenericProvisioningPDU,
    ) -> Result<Option<ProvisioningPDU>, DeviceError> {
        match pdu {
            GenericProvisioningPDU::TransactionStart(transaction_start) => {
                if self.check_ack(ctx).await? {
                    return Ok(None);
                }

                if transaction_start.seg_n == 0 {
                    let pdu = ProvisioningPDU::parse(&*transaction_start.data)?;
                    self.do_ack(ctx).await?;
                    Ok(Some(pdu))
                } else {
                    if let None = &mut self.inbound_segments {
                        if let Ok(segments) = InboundSegments::new(
                            transaction_start.seg_n,
                            &transaction_start.data,
                        ) {
                            self.inbound_segments.replace(segments);
                        }
                    }
                    Ok(None)
                }
            }
            GenericProvisioningPDU::TransactionContinuation(transaction_continuation) => {
                if let Some(segments) = &mut self.inbound_segments {
                    if let Ok(Some(provisioning_pdu)) = segments.receive(
                        transaction_continuation.segment_index,
                        &transaction_continuation.data,
                    ) {
                        Ok(Some(provisioning_pdu))
                    } else {
                        Ok(None)
                    }
                } else {
                    // wait to see the TransactionStart again
                    Ok(None)
                }
            }
            GenericProvisioningPDU::TransactionAck => {
                // Not applicable for this role
                Ok(None)
            }
            _ => {
                // Shouldn't get here, but whatevs.
                Ok(None)
            }
        }
    }

    fn already_acked<C: TransactionContext>(&self, ctx: &mut C) -> bool {
        if let Some(ack) = self.inbound_acks {
            return ctx.transaction_number().unwrap() <= ack;
        } else {
            false
        }
    }

    async fn check_ack<C: TransactionContext>(&mut self, ctx: &mut C) -> Result<bool, DeviceError> {
        if !self.already_acked(ctx) {
            Ok(false)
        } else {
            self.do_ack(ctx).await?;
            Ok(true)
        }
    }

    async fn do_ack<C: TransactionContext>(&self, ctx: &mut C) -> Result<(), DeviceError> {
        ctx.transmit_transaction_ack().await?;
        let mut acks = self.inbound_acks;
        match acks {
            None => {
                acks = *ctx.transaction_number();
            }
            Some(latest) => {
                // TODO fix wraparound rollover.
                if ctx.transaction_number().unwrap() > latest {
                    acks = *ctx.transaction_number();
                }
            }
        }
        Ok(())
    }
}


struct InboundSegments {
    segments: Vec<Option<Vec<u8, 64>>, 32>,
}

impl InboundSegments {
    fn new(
        seg_n: u8,
        data: &Vec<u8, 64>,
    ) -> Result<Self, InsufficientBuffer> {
        let mut this = Self {
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
        segment_index: u8,
        data: &Vec<u8, 64>,
    ) -> Result<Option<ProvisioningPDU>, DeviceError> {
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