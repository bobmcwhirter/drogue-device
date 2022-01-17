use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::mesh::MeshContext;
use crate::actors::ble::mesh::pipeline::segmentation::outbound::{
    OutboundSegments, OutboundSegmentsIter,
};
use crate::actors::ble::mesh::pipeline::segmentation::Segmentation;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::generic_provisioning::{
    GenericProvisioningPDU, ProvisioningBearerControl,
};
use crate::drivers::ble::mesh::provisioning::ProvisioningPDU;
use core::future::Future;
use core::iter::Iterator;
use heapless::Vec;

pub struct ProvisioningBearer {
    segmentation: Segmentation,
    link_id: Option<u32>,
    inbound_transaction_number: Option<u8>,
    acked_inbound_transaction_number: Option<u8>,
    outbound_pdu: Option<OutboundPDU>,
    outbound_transaction_number: u8,
}

impl Default for ProvisioningBearer {
    fn default() -> Self {
        Self {
            segmentation: Segmentation::default(),
            link_id: None,
            inbound_transaction_number: None,
            acked_inbound_transaction_number: None,
            outbound_pdu: None,
            outbound_transaction_number: 0x80,
        }
    }
}

impl ProvisioningBearer {
    pub async fn process_inbound<C: MeshContext>(
        &mut self,
        ctx: &C,
        pdu: PDU,
    ) -> Result<Option<ProvisioningPDU>, DeviceError> {
        match pdu.pdu {
            GenericProvisioningPDU::ProvisioningBearerControl(pbc) => {
                match pbc {
                    ProvisioningBearerControl::LinkOpen(uuid) => {
                        if ctx.uuid() == uuid {
                            if let None = self.link_id {
                                self.inbound_transaction_number
                                    .replace(pdu.transaction_number);
                                self.link_id.replace(pdu.link_id);

                                ctx.transmit_pdu(PDU {
                                    link_id: pdu.link_id,
                                    transaction_number: 0,
                                    pdu: GenericProvisioningPDU::ProvisioningBearerControl(ProvisioningBearerControl::LinkAck),
                                })
                                .await;
                                Ok(None)
                            } else {
                                Err(DeviceError::InvalidLink)
                            }
                        } else {
                            Ok(None)
                        }
                    }
                    ProvisioningBearerControl::LinkAck => {
                        /* not applicable for this role */
                        Ok(None)
                    }
                    ProvisioningBearerControl::LinkClose(reason) => {
                        self.link_id.take();
                        self.inbound_transaction_number.take();
                        Ok(None)
                    }
                }
            }
            GenericProvisioningPDU::TransactionStart(_)
            | GenericProvisioningPDU::TransactionContinuation(_) => {
                let result = self.segmentation.process_inbound(pdu.pdu).await;
                if let Ok(Some(_)) = result {
                    self.ack_transaction(ctx).await?;
                }
                result
            }
            GenericProvisioningPDU::TransactionAck => Ok(None),
        }
    }

    async fn ack_transaction<C: MeshContext>(
        &mut self,
        ctx: &C,
    ) -> Result<bool, DeviceError> {
        match (
            self.inbound_transaction_number,
            self.acked_inbound_transaction_number,
        ) {
            // TODO dry up this repetition
            (Some(current), Some(last_ack)) if current > last_ack => {
                ctx.transmit_pdu(PDU {
                    link_id: self.link_id.ok_or(DeviceError::InvalidLink)?,
                    transaction_number: self
                        .inbound_transaction_number
                        .ok_or(DeviceError::InvalidTransactionNumber)?,
                    pdu: GenericProvisioningPDU::TransactionAck,
                })
                .await?;
                self.acked_inbound_transaction_number.replace(current);
                self.inbound_transaction_number.take();
                Ok(true)
            }
            (Some(current), None) => {
                ctx.transmit_pdu(PDU {
                    link_id: self.link_id.ok_or(DeviceError::InvalidLink)?,
                    transaction_number: self
                        .inbound_transaction_number
                        .ok_or(DeviceError::InvalidTransactionNumber)?,
                    pdu: GenericProvisioningPDU::TransactionAck,
                })
                .await?;
                self.acked_inbound_transaction_number.replace(current);
                self.inbound_transaction_number.take();
                Ok(true)
            }
            _ => Err(DeviceError::InvalidTransactionNumber),
        }
    }

    pub async fn process_outbound(
        &mut self,
        pdu: ProvisioningPDU,
    ) -> Result<impl Iterator<Item=PDU> + '_, DeviceError> {
            let segments = self.segmentation.process_outbound(pdu).await;

        let transaction_number = self.outbound_transaction_number;
        self.outbound_transaction_number = self.outbound_transaction_number + 1;

        self.outbound_pdu.replace( OutboundPDU {
            link_id: self.link_id.ok_or(DeviceError::InvalidLink)?,
            transaction_number,
            segments: segments,
        } );

        Ok(self.outbound_pdu.as_mut().unwrap().iter())
    }
}

pub struct OutboundPDU {
    link_id: u32,
    transaction_number: u8,
    segments: OutboundSegments,
}

impl OutboundPDU {
    pub fn iter(&self) -> OutboundPDUIter {
        OutboundPDUIter {
            link_id: self.link_id,
            transaction_number: self.transaction_number,
            inner: self.segments.iter(),
        }
    }
}

pub struct OutboundPDUIter<'i> {
    link_id: u32,
    transaction_number: u8,
    inner: OutboundSegmentsIter<'i>,
}

impl<'i> OutboundPDUIter<'i> {
    fn new(inner: OutboundSegmentsIter<'i>, link_id: u32, transaction_number: u8) -> Self {
        Self {
            link_id,
            transaction_number,
            inner,
        }
    }
}

impl<'i> Iterator for OutboundPDUIter<'i> {
    type Item = PDU;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.inner.next();
        match inner {
            None => None,
            Some(inner) => Some(PDU {
                link_id: self.link_id,
                transaction_number: self.transaction_number,
                pdu: GenericProvisioningPDU::TransactionAck,
            }),
        }
    }
}
