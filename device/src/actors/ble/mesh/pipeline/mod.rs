use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::mesh::{Mesh, MeshData};
use crate::actors::ble::mesh::pipeline::provisionable::{Provisionable, ProvisionableContext};
use crate::actors::ble::mesh::pipeline::provisioning_bearer::ProvisioningBearer;
use crate::actors::ble::mesh::pipeline::transaction::Transaction;

mod mesh;
mod provisionable;
mod provisioning_bearer;
mod transaction;

pub trait PipelineContext: ProvisionableContext {}

pub struct Pipeline {
    mesh: Mesh,
    provisioning_bearer: ProvisioningBearer,
    transaction: Transaction,
    provisionable: Provisionable,
}

impl Pipeline {
    async fn process<C: PipelineContext>(
        &mut self,
        ctx: &mut C,
        data: &[u8],
    ) -> Result<(), DeviceError> {
        if let Some(result) = self.mesh.process(ctx, &data).await? {
            match result {
                MeshData::Provisioning(pdu) => {
                    if let Some(generic_provisioning_pdu) =
                        self.provisioning_bearer.process(ctx, pdu).await?
                    {
                        if let Some(provisioning_pdu) = self
                            .transaction
                            .process(ctx, generic_provisioning_pdu)
                            .await?
                        {
                            self.provisionable.process(ctx, provisioning_pdu).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
