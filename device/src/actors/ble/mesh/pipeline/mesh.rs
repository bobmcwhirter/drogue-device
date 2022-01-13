use crate::actors::ble::mesh::device::DeviceError;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::PB_ADV;

pub trait MeshContext {
    fn uuid(&self) -> Uuid;
}

pub struct Mesh {

}

pub enum MeshData {
    Provisioning(PDU),
}

impl Mesh {
    pub async fn process<C: MeshContext>(&mut self, ctx: &mut C, data: &[u8]) -> Result<Option<MeshData>, DeviceError> {
        if data.len() >= 2 {
            if data[1] == PB_ADV {
                Ok(Some(MeshData::Provisioning(PDU::parse(data).map_err(|_| DeviceError::InvalidPacket)?)))
            } else {
                Err(DeviceError::InvalidPacket)
            }
        } else {
            Ok(None)
        }
    }
}