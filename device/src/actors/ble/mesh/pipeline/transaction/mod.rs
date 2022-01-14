use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::provisioning_bearer::ProvisioningBearerContext;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::generic_provisioning::{GenericProvisioningPDU, TransactionContinuation, TransactionStart};
use crate::drivers::ble::mesh::provisioning::ProvisioningPDU;
use crate::drivers::ble::mesh::InsufficientBuffer;
use self::inbound::InboundSegments;
use self::outbound::OutboundSegments;
use core::future::Future;
use core::sync::atomic::{AtomicBool, Ordering};
use heapless::Vec;

mod inbound;
mod outbound;

pub trait TransactionContext: ProvisioningBearerContext {

    fn transmit_transaction_ack<'m>(&'m self) -> Self::GenericProvisioningPduFuture<'m> {
        self.transmit_generic_provisioning_pdu(&GenericProvisioningPDU::TransactionAck)
    }

    type GenericProvisioningPduFuture<'m>: Future<Output = Result<(), DeviceError>>
    where
        Self: 'm;

    fn transmit_generic_provisioning_pdu<'m>(
        &'m self,
        pdu: &'m GenericProvisioningPDU,
    ) -> Self::GenericProvisioningPduFuture<'m>;
}

pub struct Transaction {
    inbound_segments: Option<InboundSegments>,
    inbound_acks: Option<u8>,
    outbound_transaction_number: u8,
    outbound_segments: Option<OutboundSegments>,
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            inbound_segments: None,
            inbound_acks: None,
            outbound_transaction_number: 0x80,
            outbound_segments: None
        }
    }
}

impl Transaction {
    pub async fn process_inbound<C: TransactionContext>(
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
                        if let Ok(segments) =
                            InboundSegments::new(transaction_start.seg_n, &transaction_start.data)
                        {
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

    pub async fn process_outbound<C: TransactionContext>(
        &mut self,
        ctx: &C,
        pdu: Option<ProvisioningPDU>,
    ) -> Result<(), DeviceError> {
        match pdu {
            None => {}
            Some(pdu) => {
                self.outbound_segments.replace(OutboundSegments::new(self.outbound_transaction_number, &pdu));
                self.outbound_transaction_number = self.outbound_transaction_number + 1;
            }
        }

        if let Some(segments) = &self.outbound_segments {
            for segment in segments.iter() {
                ctx.transmit_generic_provisioning_pdu(&segment).await?;
            }
        }
        Ok(())
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
