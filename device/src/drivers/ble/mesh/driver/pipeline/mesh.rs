use crate::drivers::ble::mesh::pdu::bearer::advertising;
use crate::drivers::ble::mesh::pdu::network;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::{MESH_MESSAGE, PB_ADV};
use core::future::Future;

pub trait MeshContext {
    fn uuid(&self) -> Uuid;

    type TransmitFuture<'m>: Future<Output = Result<(), DeviceError>>
    where
        Self: 'm;

    fn transmit_pdu<'m>(&'m self, pdu: advertising::PDU) -> Self::TransmitFuture<'m>;
}

pub struct Mesh {}

pub enum MeshData {
    Provisioning(advertising::PDU),
    Network(network::AuthenticatedPDU),
}

impl Default for Mesh {
    fn default() -> Self {
        Self {}
    }
}

#[allow(unused_variables)]
impl Mesh {
    pub async fn process_inbound<C: MeshContext>(
        &mut self,
        ctx: &C,
        data: &[u8],
    ) -> Result<Option<MeshData>, DeviceError> {
        if data.len() >= 2 {
            if data[1] == PB_ADV {
                Ok(Some(MeshData::Provisioning(
                    advertising::PDU::parse(data).map_err(|_| DeviceError::InvalidPacket)?,
                )))
            } else if data[1] == MESH_MESSAGE {
                Ok(Some(MeshData::Network(
                    network::AuthenticatedPDU::parse(data).map_err(|_| DeviceError::InvalidPacket)?,
                )))
            } else {
                Err(DeviceError::InvalidPacket)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn process_outbound<C: MeshContext>(
        &mut self,
        ctx: &C,
        pdu: advertising::PDU,
    ) -> Result<(), DeviceError> {
        ctx.transmit_pdu(pdu).await
    }
}
