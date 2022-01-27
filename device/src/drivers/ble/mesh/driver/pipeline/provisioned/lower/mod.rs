use core::convert::TryInto;
use ccm::aead::Buffer;
use cmac::NewMac;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::network::AuthenticatedPDU;
use crate::drivers::ble::mesh::pdu::{lower, network, upper};
use crate::drivers::ble::mesh::pdu::lower::{Access, AccessMessage, ControlMessage, PDU};

use heapless::Vec;
use crate::drivers::ble::mesh::driver::pipeline::provisioned::network::authentication::AuthenticationContext;

pub trait LowerContext : AuthenticationContext {
    fn decrypt_device_key(&self, nonce: &[u8], bytes: &mut [u8], mic: &[u8]) -> Result<(), DeviceError>;
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
                        defmt::info!("unsegmented access");
                        defmt::info!("akf {}", access.akf);
                        defmt::info!("aid {}", access.aid);
                        // TransMIC is 32 bits for unsegmented access messages.
                        defmt::info!("split lower {:x}", payload);
                        let (payload, trans_mic) = payload.split_at( payload.len() - 4);
                        let mut payload = Vec::from_slice(payload).map_err(|_|DeviceError::InsufficientBuffer)?;
                        //let trans_mic = TransMIC::Bit32( trans_mic.try_into().map_err(|_|DeviceError::InvalidKeyLength)? );
                        let mut nonce = [0;13];

                        if access.akf {
                            // decrypt with aid key
                        }  else {
                            // nonce types
                            nonce[0] = 0x02;
                            // aszmic + padd
                            nonce[1] = 0;

                            // sequence
                            let seq = pdu.seq.to_be_bytes();
                            nonce[2] = seq[1];
                            nonce[3] = seq[2];
                            nonce[4] = seq[3];

                            // src
                            let src = pdu.src.as_bytes();
                            nonce[5] = src[0];
                            nonce[6] = src[1];

                            // dst
                            let dst = pdu.dst.as_bytes();
                            nonce[7] = dst[0];
                            nonce[8] = dst[1];

                            // iv index
                            let iv_index = ctx.iv_index().ok_or(DeviceError::InvalidState)?;
                            let iv_index = iv_index.to_be_bytes();
                            nonce[9] = iv_index[0];
                            nonce[10] = iv_index[1];
                            nonce[11] = iv_index[2];
                            nonce[12] = iv_index[3];

                            // decrypt with device key
                            ctx.decrypt_device_key(&nonce, &mut payload, &trans_mic);
                        }
                        Ok(Some(upper::PDU::Access( upper::Access {
                            payload,
                            //trans_mic,
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