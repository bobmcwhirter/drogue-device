use core::convert::TryInto;
use ccm::aead::Buffer;
use cmac::NewMac;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::network::AuthenticatedPDU;
use crate::drivers::ble::mesh::pdu::{lower, network, upper};
use crate::drivers::ble::mesh::pdu::lower::{Access, AccessMessage, ControlMessage, PDU};

use heapless::Vec;
use crate::drivers::ble::mesh::crypto::nonce::DeviceNonce;
use crate::drivers::ble::mesh::driver::pipeline::provisioned::network::authentication::AuthenticationContext;

pub trait LowerContext : AuthenticationContext {
    fn decrypt_device_key(&self, nonce: DeviceNonce, bytes: &mut [u8], mic: &[u8]) -> Result<(), DeviceError>;
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

    pub async fn process_inbound<C:LowerContext>(&mut self, ctx:&C, pdu: AuthenticatedPDU) -> Result<Option<upper::PDU>, DeviceError> {
        match pdu.transport_pdu {
            PDU::Access(access) => {
                match access.message {
                    AccessMessage::Unsegmented(payload) => {
                        // TransMIC is 32 bits for unsegmented access messages.
                        let (payload, trans_mic) = payload.split_at( payload.len() - 4);
                        let mut payload = Vec::from_slice(payload).map_err(|_|DeviceError::InsufficientBuffer)?;

                        if access.akf {
                            // decrypt with aid key
                        }  else {
                            // decrypt with device key
                            let nonce = DeviceNonce::new(false, pdu.seq, pdu.src, pdu.dst, ctx.iv_index().ok_or(DeviceError::CryptoError)?);
                            ctx.decrypt_device_key(nonce, &mut payload, &trans_mic);
                        }
                        Ok(Some(upper::PDU::Access( upper::Access {
                            payload,
                        })))
                    }
                    AccessMessage::Segmented { .. } => {
                        defmt::info!("segmented access");
                        todo!()
                    }
                }
            }
            PDU::Control(control) => {
                match control.message {
                    ControlMessage::Unsegmented { .. } => {
                        defmt::info!("unsegmented control");
                        todo!()
                    }
                    ControlMessage::Segmented { .. } => {
                        defmt::info!("segmented control");
                        todo!()
                    }
                }
            }
        }
    }
}