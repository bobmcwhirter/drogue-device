use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::mesh::{Mesh, MeshData};
use crate::actors::ble::mesh::pipeline::provisionable::{Provisionable, ProvisionableContext};
use crate::actors::ble::mesh::pipeline::provisioning_bearer::ProvisioningBearer;
use crate::actors::ble::mesh::pipeline::segmentation::Segmentation;
use crate::drivers::ble::mesh::provisioning::{Capabilities, ProvisioningPDU};
use heapless::Vec;

pub mod mesh;
pub mod provisionable;
pub mod provisioning_bearer;
pub mod segmentation;

pub trait PipelineContext: ProvisionableContext {}

pub struct Pipeline {
    mesh: Mesh,
    provisioning_bearer: ProvisioningBearer,
    provisionable: Provisionable,
}

impl Pipeline {
    pub fn new(capabilities: Capabilities) -> Self {
        Self {
            mesh: Default::default(),
            provisioning_bearer: Default::default(),
            provisionable: Provisionable::new(capabilities),
        }
    }

    pub async fn process_inbound<C: PipelineContext>(
        &mut self,
        ctx: &C,
        data: &[u8],
    ) -> Result<(), DeviceError> {
        if let Some(result) = self.mesh.process_inbound(ctx, &data).await? {
            match result {
                MeshData::Provisioning(pdu) => {
                    if let Some(provisioning_pdu) =
                        self.provisioning_bearer.process_inbound(ctx, pdu).await?
                    {
                        if let Some(outbound) = self
                            .provisionable
                            .process_inbound(ctx, provisioning_pdu)
                            .await? {

                            for pdu in self.provisioning_bearer.process_outbound(outbound).await? {
                                self.mesh.process_outbound(ctx, pdu).await?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
