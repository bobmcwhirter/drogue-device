use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::{access, lower, upper};
use crate::drivers::ble::mesh::pdu::access::AccessMessage;
use crate::drivers::ble::mesh::pdu::upper::PDU;

pub trait UpperContext {

}

pub struct Upper {

}

impl Default for Upper {
    fn default() -> Self {
        Self {

        }
    }
}

impl Upper {
    pub async fn process_inbound<C:UpperContext>(&mut self, ctx: &C, pdu: upper::PDU) -> Result<Option<AccessMessage>, DeviceError> {
        match pdu {
            PDU::Control(control) => {
                todo!()
            }
            PDU::Access(access) => {
                // todo: check trans_mic
                let message = AccessMessage::parse(&*access.payload)?;
                Ok(Some(message))
            }
        }
    }
}
