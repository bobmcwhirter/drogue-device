use crate::actors::ble::mesh::device::Device;
use crate::actors::ble::mesh::handlers::transcript::Transcript;
use crate::drivers::ble::mesh::crypto::{aes_ccm_decrypt, aes_cmac, s1};
use crate::drivers::ble::mesh::provisioning::{
    Confirmation, InputOOBAction, OOBAction, OOBSize, OutputOOBAction, ProvisioningPDU, PublicKey,
    Random, Start,
};
use crate::drivers::ble::mesh::transport::Transport;
use core::convert::TryFrom;
use core::marker::PhantomData;
use heapless::Vec;
use p256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
use p256::EncodedPoint;
use rand_core::RngCore;

enum State {
    None,
    Invite,
    Capabilities,
    Start,
    PublicKey,
    InputComplete,
    Confirmation,
    Random,
    Data,
    Complete,
    Failed,
}

pub enum AuthValue {
    None,
    InputEvents(u32),
    OutputEvents(u32),
    InputNumeric(u32),
    OutputNumeric(u32),
    InputAlphanumeric(Vec<u8, 8>),
    OutputAlphanumeric(Vec<u8, 8>),
}

impl AuthValue {
    pub fn get_bytes(&self) -> [u8; 16] {
        let mut bytes = [0; 16];
        match self {
            AuthValue::None => {
                // all zeros
            }
            AuthValue::InputEvents(num)
            | AuthValue::OutputEvents(num)
            | AuthValue::InputNumeric(num)
            | AuthValue::OutputNumeric(num) => {
                let num_bytes = num.to_be_bytes();
                bytes[12] = num_bytes[0];
                bytes[13] = num_bytes[1];
                bytes[14] = num_bytes[2];
                bytes[15] = num_bytes[3];
            }
            AuthValue::InputAlphanumeric(chars) | AuthValue::OutputAlphanumeric(chars) => {
                for (i, byte) in chars.iter().enumerate() {
                    bytes[i] = *byte
                }
            }
        }

        bytes
    }
}

pub struct ProvisioningHandler<T, R>
where
    T: Transport + 'static,
    R: RngCore,
{
    state: State,
    transcript: Transcript,
    auth_value: Option<AuthValue>,
    random_provisioner: Option<[u8;16]>,
    _marker: PhantomData<(T, R)>,
}

impl<T, R> ProvisioningHandler<T, R>
where
    T: Transport + 'static,
    R: RngCore,
{
    pub(crate) fn new() -> Self {
        Self {
            state: State::None,
            transcript: Transcript::new(),
            auth_value: None,
            random_provisioner: None,
            _marker: PhantomData,
        }
    }

    pub(crate) async fn handle(
        &mut self,
        device: &Device<T, R>,
        mut pdu: ProvisioningPDU,
    ) -> Result<(), ()> {
        match pdu {
            ProvisioningPDU::Invite(invite) => {
                defmt::info!(">> ProvisioningPDU::Invite");
                self.state = State::Invite;
                self.transcript.add_invite(&invite)?;
                device.tx_capabilities()?;
                self.transcript.add_capabilities(&device.capabilities)?;
            }
            ProvisioningPDU::Capabilities(_) => {}
            ProvisioningPDU::Start(start) => {
                defmt::info!(">> ProvisioningPDU::Start");
                self.transcript.add_start(&start)?;
                let auth_value = self.determine_auth_value(device, &start)?;
                // TODO actually let the device/app/thingy know what it is so that it can blink/flash/accept input
                self.auth_value.replace(auth_value);
            }
            ProvisioningPDU::PublicKey(public_key) => {
                defmt::info!(">> ProvisioningPDU::PublicKey");
                self.transcript.add_pubkey_provisioner(&public_key)?;
                // TODO remove unwrap
                let peer_pk =
                    p256::PublicKey::from_encoded_point(&EncodedPoint::from_affine_coordinates(
                        &public_key.x.into(),
                        &public_key.y.into(),
                        false,
                    ))
                    .unwrap();
                device.key_manager.set_peer_public_key(peer_pk);
                let pk = device.public_key();
                let xy = pk.to_encoded_point(false);
                let x = xy.x().unwrap();
                let y = xy.y().unwrap();
                let pk = PublicKey {
                    x: <[u8; 32]>::try_from(x.as_slice()).map_err(|_| ())?,
                    y: <[u8; 32]>::try_from(y.as_slice()).map_err(|_| ())?,
                };
                self.transcript.add_pubkey_device(&pk)?;
                device.tx_provisioning_pdu(ProvisioningPDU::PublicKey(pk));
            }
            ProvisioningPDU::InputComplete => {
                defmt::info!(">> ProvisioningPDU::InputComplete");
            }
            ProvisioningPDU::Confirmation(confirmation) => {
                defmt::info!(">> ProvisioningPDU::Confirmation {}", confirmation);
                let confirmation_device = self.confirmation_device(device)?;
                device.tx_provisioning_pdu(ProvisioningPDU::Confirmation(confirmation_device));
            }
            ProvisioningPDU::Random(random) => {
                defmt::info!(">> ProvisioningPDU::Random");
                device.tx_provisioning_pdu(ProvisioningPDU::Random(Random {
                    random: device.key_manager.random,
                }));
                self.random_provisioner.replace( random.random );
            }
            ProvisioningPDU::Data(ref mut data) => {
                defmt::info!(">> ProvisioningPDU::Data");

                let mut provisioning_salt = [0; 48];
                provisioning_salt[0..16].copy_from_slice( &self.transcript.confirmation_salt()?.into_bytes());
                provisioning_salt[16..32].copy_from_slice( self.random_provisioner.as_ref().unwrap() );
                provisioning_salt[32..48].copy_from_slice( &device.key_manager.random );
                let provisioning_salt = &s1( &provisioning_salt )?.into_bytes()[0..];

                defmt::info!("prov-salt {:x}", provisioning_salt);

                let session_key = &device.key_manager.k1( &provisioning_salt, b"prsk")?.into_bytes()[0..];
                let session_nonce = &device.key_manager.k1( &provisioning_salt, b"prsn")?.into_bytes()[3..];

                defmt::info!("session key {:x}", session_key);
                defmt::info!("session nonce {:x}", session_nonce);
                defmt::info!("encrypted {:x}", data.encrypted);

                //let mut buffer = CryptoBuffer::<128>::from_slice(&data.encrypted)?;
                //let result = aes_ccm_decrypt(&session_key, &session_nonce, &mut buffer, &data.mic);
                let result = aes_ccm_decrypt(&session_key, &session_nonce, &mut data.encrypted, &data.mic);
                match result {
                    Ok(_) => {
                        defmt::info!("decrypted!");
                    }
                    Err(_) => {
                        defmt::info!("decryption error!");
                    }
                }
                device.tx_provisioning_pdu(ProvisioningPDU::Complete);
            }
            ProvisioningPDU::Complete => {
                defmt::info!(">> ProvisioningPDU::Complete");
            }
            ProvisioningPDU::Failed(_failed) => {
                defmt::info!(">> ProvisioningPDU::Failed");
            }
        }
        Ok(())
    }

    fn confirmation_device(&self, device: &Device<T, R>) -> Result<Confirmation, ()> {
        defmt::info!("confirmation_device A");
        let salt = self.transcript.confirmation_salt()?;
        defmt::info!("confirmation_device B");
        let confirmation_key = device.key_manager.k1(&*salt.into_bytes(), b"prck")?;
        defmt::info!("confirmation_device C");
        let mut bytes: Vec<u8, 32> = Vec::new();
        defmt::info!("confirmation_device D");
        bytes.extend_from_slice(&device.key_manager.random)?;
        defmt::info!("confirmation_device E");
        bytes.extend_from_slice(&self.auth_value.as_ref().ok_or(())?.get_bytes())?;
        defmt::info!("confirmation_device F");
        let confirmation_device = aes_cmac(&confirmation_key.into_bytes(), &bytes)?;
        defmt::info!("confirmation_device G");

        let mut confirmation = [0; 16];
        defmt::info!("confirmation_device H");
        for (i, byte) in confirmation_device.into_bytes().iter().enumerate() {
            confirmation[i] = *byte;
        }
        defmt::info!("confirmation_device I");

        Ok(Confirmation { confirmation })
    }

    fn determine_auth_value(&mut self, device: &Device<T, R>, start: &Start) -> Result<AuthValue,()> {
        Ok(match (&start.authentication_action, &start.authentication_size) {
            (
                OOBAction::Output(OutputOOBAction::Blink)
                | OOBAction::Output(OutputOOBAction::Beep)
                | OOBAction::Output(OutputOOBAction::Vibrate),
                OOBSize::MaximumSize(size),
            ) => {
                let auth_raw = self.random_physical_oob(&device, *size);
                AuthValue::OutputEvents(auth_raw)
            }
            (
                OOBAction::Input(InputOOBAction::Push) | OOBAction::Input(InputOOBAction::Twist),
                OOBSize::MaximumSize(size),
            ) => {
                let auth_raw = self.random_physical_oob(&device, *size);
                AuthValue::InputEvents(auth_raw)
            }
            (OOBAction::Output(OutputOOBAction::OutputNumeric), OOBSize::MaximumSize(size)) => {
                let auth_raw = self.random_numeric(&device, *size);
                AuthValue::OutputNumeric(auth_raw)
            }
            // TODO actually dispatch to device/app/thing's UI for inputs instead of just making up shit.
            (OOBAction::Input(InputOOBAction::InputNumeric), OOBSize::MaximumSize(size)) => {
                let auth_raw = self.random_numeric(&device, *size);
                AuthValue::InputNumeric(auth_raw)
            }
            (
                OOBAction::Output(OutputOOBAction::OutputAlphanumeric),
                OOBSize::MaximumSize(size),
            ) => {
                let auth_raw = self.random_alphanumeric(&device, *size)?;
                AuthValue::OutputAlphanumeric(auth_raw)
            }
            (OOBAction::Input(InputOOBAction::InputAlphanumeric), OOBSize::MaximumSize(size)) => {
                let auth_raw = self.random_alphanumeric(&device, *size)?;
                AuthValue::InputAlphanumeric(auth_raw)
            }
            _ => {
                // zeros!
                AuthValue::None
            }
        })
    }

    fn random_physical_oob(&self, device: &Device<T, R>, size: u8) -> u32 {
        // "select a random integer between 0 and 10 to the power of the Authentication Size exclusive"
        //
        // ... which could be an absolute metric tonne of beeps/twists/pushes if AuthSize is large-ish.
        let mut max = 1;
        for _ in 0..size {
            max = max * 10;
        }

        loop {
            let candidate = device.next_random_u32();
            if candidate > 0 && candidate < max {
                return candidate;
            }
        }
    }

    fn random_numeric(&self, device: &Device<T, R>, size: u8) -> u32 {
        loop {
            let candidate = device.next_random_u32();

            match size {
                1 => {
                    if candidate < 10 {
                        return candidate;
                    }
                }
                2 => {
                    if candidate < 100 {
                        return candidate;
                    }
                }
                3 => {
                    if candidate < 1_000 {
                        return candidate;
                    }
                }
                4 => {
                    if candidate < 10_000 {
                        return candidate;
                    }
                }
                5 => {
                    if candidate < 100_000 {
                        return candidate;
                    }
                }
                6 => {
                    if candidate < 1_000_000 {
                        return candidate;
                    }
                }
                7 => {
                    if candidate < 10_000_000 {
                        return candidate;
                    }
                }
                8 => {
                    if candidate < 100_000_000 {
                        return candidate;
                    }
                }
                _ => {
                    // should never get here, but...
                    return 0;
                }
            }
        }
    }

    fn random_alphanumeric(&self, device: &Device<T, R>, size: u8) -> Result<Vec<u8, 8>, ()> {
        let mut random = Vec::new();
        for _ in 0..size {
            loop {
                let candidate = device.next_random_u8();
                if candidate >= 64 && candidate <= 90 {
                    // Capital ASCII letters A-Z
                    random.push(candidate).map_err(|_|())?;
                } else if candidate >= 48 && candidate <= 57 {
                    // ASCII numbers 0-9
                    random.push(candidate).map_err(|_|())?;
                }
            }
        }
        Ok(random)
    }
}
