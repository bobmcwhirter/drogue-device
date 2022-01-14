use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::mesh::{Mesh, MeshData};
use crate::actors::ble::mesh::pipeline::provisionable::{Provisionable, ProvisionableContext};
use crate::actors::ble::mesh::pipeline::provisioning_bearer::ProvisioningBearer;
use crate::actors::ble::mesh::pipeline::transaction::Transaction;
use crate::drivers::ble::mesh::provisioning::{Capabilities, ProvisioningPDU};
use heapless::Vec;

pub mod mesh;
pub mod provisionable;
pub mod provisioning_bearer;
pub mod transaction;

pub trait PipelineContext: ProvisionableContext {}

pub struct Pipeline {
    mesh: Mesh,
    provisioning_bearer: ProvisioningBearer,
    transaction: Transaction,
    provisionable: Provisionable,
}

impl Pipeline {

    pub fn new(capabilities: Capabilities) -> Self {
        Self {
            mesh: Default::default(),
            provisioning_bearer: Default::default(),
            transaction: Default::default(),
            provisionable: Provisionable::new(capabilities),
        }

    }

    pub async fn process_inbound<C: PipelineContext>(
        &mut self,
        ctx: &mut C,
        data: &[u8],
    ) -> Result<(), DeviceError> {
        if let Some(result) = self.mesh.process_inbound(ctx, &data).await? {
            match result {
                MeshData::Provisioning(pdu) => {
                    if let Some(generic_provisioning_pdu) =
                        self.provisioning_bearer.process_inbound(ctx, pdu).await?
                    {
                        if let Some(provisioning_pdu) = self
                            .transaction
                            .process_inbound(ctx, generic_provisioning_pdu)
                            .await?
                        {
                            let outbound = self
                                .provisionable
                                .process_inbound(ctx, provisioning_pdu)
                                .await?;
                            self.transaction.process_outbound(ctx, outbound).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
