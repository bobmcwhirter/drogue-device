use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::mesh::MeshContext;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::generic_provisioning::{
    GenericProvisioningPDU, ProvisioningBearerControl,
};
use core::future::Future;

pub trait ProvisioningBearerContext: MeshContext {
    fn link_id(&mut self) -> &mut Option<u32>;
    fn transaction_number(&mut self) -> &mut Option<u8>;

    type TransmitFuture<'m>: Future<Output = ()>
    where
        Self: 'm;

    fn transmit_pdu<'m>(&mut self, pdu: PDU) -> Self::TransmitFuture<'m>;
}

pub struct ProvisioningBearer {}

impl ProvisioningBearer {
    pub async fn process<C: ProvisioningBearerContext>(
        &mut self,
        ctx: &mut C,
        pdu: PDU,
    ) -> Result<Option<GenericProvisioningPDU>, DeviceError> {
        match pdu.pdu {
            GenericProvisioningPDU::ProvisioningBearerControl(pbc) => {
                match pbc {
                    ProvisioningBearerControl::LinkOpen(uuid) => {
                        if ctx.uuid() == uuid {
                            if let None = ctx.link_id() {
                                ctx.link_id().replace(pdu.link_id);
                                ctx.transmit_pdu( PDU {
                                    link_id: pdu.link_id,
                                    transaction_number: 0,
                                    pdu: GenericProvisioningPDU::TransactionAck
                                }).await;
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
                        ctx.link_id().take();
                        ctx.transaction_number().take();
                        Ok(None)
                    }
                }
            }
            _ => {
                ctx.transaction_number().replace(pdu.transaction_number);
                Ok(Some(pdu.pdu))
            },
        }
    }
}
