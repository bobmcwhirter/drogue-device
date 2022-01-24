mod network_message_cache;
mod obfuscation;

use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::{lower, network};

pub trait NetworkContext {
    fn authenticate(&self, nid: u8, data: &[u8], net_mic: &[u8]) -> Option<&[u8]> {
        None
    }

}

pub struct Network {

}

impl Network {

    pub async fn process_inbound<C:NetworkContext>(&mut self, ctx: &C, pdu: network::AuthenticatedPDU) -> Result<Option<lower::PDU>, DeviceError> {
        /*
        if let Some(decrypted) = ctx.authenticate(pdu.nid, pdu.transport_pdu, pdu.net_mic) {
            Err(DeviceError::InvalidPacket)
        } else {
            Ok(None)
        }
         */
        todo!()
    }

}