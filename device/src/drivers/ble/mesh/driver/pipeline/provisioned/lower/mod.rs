use core::convert::TryInto;
use ccm::aead::Buffer;
use cmac::NewMac;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::network::AuthenticatedPDU;
use crate::drivers::ble::mesh::pdu::{lower, upper};
use crate::drivers::ble::mesh::pdu::lower::{Access, AccessMessage, ControlMessage, PDU};

use heapless::Vec;
use crate::drivers::ble::mesh::pdu::upper::TransMIC;

pub trait LowerContext {

}

pub struct Lower {

}

impl Default for Lower {
    fn default() -> Self {
        Self {

        }
    }
}

impl Lower {

    pub async fn process_inbound<C:LowerContext>(&mut self, ctx:&C, pdu: lower::PDU) -> Result<Option<upper::PDU>, DeviceError> {
        match pdu {
            PDU::Access(access) => {
                match access.message {
                    AccessMessage::Unsegmented(payload) => {
                        // TransMIC is 32 bits for unsegmented access messages.
                        let (payload, trans_mic) = payload.split_at( payload.len() - 4);
                        let payload = Vec::from_slice(payload).map_err(|_|DeviceError::InsufficientBuffer)?;
                        let trans_mic = TransMIC::Bit32( trans_mic.try_into().map_err(|_|DeviceError::InvalidKeyLength)? );
                        Ok(Some(upper::PDU::Access( upper::Access {
                            payload,
                            trans_mic,
                        })))
                    }
                    AccessMessage::Segmented { .. } => {
                        todo!()
                    }
                }
            }
            PDU::Control(control) => {
                match control.message {
                    ControlMessage::Unsegmented { .. } => {
                        todo!()
                    }
                    ControlMessage::Segmented { .. } => {
                        todo!()
                    }
                }
            }
        }
    }
}