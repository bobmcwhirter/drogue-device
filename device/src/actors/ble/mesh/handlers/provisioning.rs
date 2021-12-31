use crate::actors::ble::mesh::device::{Device, DeviceError};
use crate::actors::ble::mesh::handlers::transcript::Transcript;
use crate::drivers::ble::mesh::crypto::{aes_ccm_decrypt, aes_cmac, s1};
use crate::drivers::ble::mesh::provisioning::{Confirmation, InputOOBAction, OOBAction, OOBSize, OutputOOBAction, ProvisioningData, ProvisioningPDU, PublicKey, Random, Start};
use crate::drivers::ble::mesh::transport::Transport;
use core::convert::TryFrom;
use core::marker::PhantomData;
use cmac::crypto_mac::InvalidKeyLength;
use heapless::Vec;
use p256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
use p256::EncodedPoint;
use rand_core::{CryptoRng, RngCore};
use crate::drivers::ble::mesh::InsufficientBuffer;
use crate::drivers::ble::mesh::storage::Storage;

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

pub struct ProvisioningHandler<T, R, S>
where
    T: Transport + 'static,
    R: RngCore,
    S: Storage + 'static,
{
    transcript: Transcript,
    auth_value: Option<AuthValue>,
    random_device: [u8; 16],
    random_provisioner: Option<[u8;16]>,
    _marker: PhantomData<(T, R, S)>,
}

impl<T, R, S> ProvisioningHandler<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    pub(crate) fn new(rng: &mut R) -> Self {
        let mut random_device = [0;16];
        rng.fill_bytes(&mut random_device);
        Self {
            transcript: Transcript::new(),
            auth_value: None,
            random_device,
            random_provisioner: None,
            _marker: PhantomData,
        }
    }

    pub(crate) async fn handle(
        &mut self,
        device: &Device<T, R, S>,
        mut pdu: ProvisioningPDU,
    ) -> Result<(), DeviceError> {
        match pdu {
            ProvisioningPDU::Invite(invite) => {
                defmt::trace!(">> Invite");
                self.transcript.add_invite(&invite)?;
                device.tx_capabilities();
                self.transcript.add_capabilities(&device.capabilities)?;
            }
            ProvisioningPDU::Capabilities(_) => {
                unimplemented!()
            }
            ProvisioningPDU::Start(start) => {
                defmt::trace!(">> Start");
                self.transcript.add_start(&start)?;
                let auth_value = self.determine_auth_value(device, &start)?;
                // TODO actually let the device/app/thingy know what it is so that it can blink/flash/accept input
                self.auth_value.replace(auth_value);
            }
            ProvisioningPDU::PublicKey(public_key) => {
                defmt::trace!(">> PublicKey");
                self.transcript.add_pubkey_provisioner(&public_key)?;
                let peer_pk_x = public_key.x;
                let peer_pk_y = public_key.y;
                defmt::trace!(">>   x = {:x}", &peer_pk_x[0..]);
                defmt::trace!(">>   y = {:x}", &peer_pk_y[0..]);

                // TODO remove unwrap
                let peer_pk =
                    p256::PublicKey::from_encoded_point(&EncodedPoint::from_affine_coordinates(
                        &peer_pk_x.into(),
                        &peer_pk_y.into(),
                        false,
                    ))
                    .unwrap();

                device.key_manager.borrow_mut().set_peer_public_key(peer_pk).await;
                let pk = device.public_key()?;
                let xy = pk.to_encoded_point(false);
                let x = xy.x().unwrap();
                let y = xy.y().unwrap();
                let pk = PublicKey {
                    x: <[u8; 32]>::try_from(x.as_slice()).map_err(|_| InsufficientBuffer)?,
                    y: <[u8; 32]>::try_from(y.as_slice()).map_err(|_| InsufficientBuffer)?,
                };
                self.transcript.add_pubkey_device(&pk)?;
                defmt::trace!("<< PublicKey");
                defmt::trace!("<<   x = {:x}", &pk.x);
                defmt::trace!("<<   y = {:x}", &pk.y);
                device.tx_provisioning_pdu(ProvisioningPDU::PublicKey(pk));
            }
            ProvisioningPDU::InputComplete => {
                defmt::trace!(">> InputComplete");
            }
            ProvisioningPDU::Confirmation(confirmation) => {
                defmt::trace!(">> Confirmation");
                defmt::trace!(">>   {}", confirmation);
                let confirmation_device = self.confirmation_device(device)?;
                device.tx_provisioning_pdu(ProvisioningPDU::Confirmation(confirmation_device));
            }
            ProvisioningPDU::Random(random) => {
                defmt::trace!(">> Random");
                defmt::trace!(">>   {}", random);
                device.tx_provisioning_pdu(ProvisioningPDU::Random(Random {
                    random: self.random_device,
                }));
                self.random_provisioner.replace( random.random );
            }
            ProvisioningPDU::Data(ref mut data) => {
                defmt::trace!(">> Data");
                defmt::trace!(">>   {}", data);

                let mut provisioning_salt = [0; 48];
                provisioning_salt[0..16].copy_from_slice( &self.transcript.confirmation_salt()?.into_bytes());
                provisioning_salt[16..32].copy_from_slice( self.random_provisioner.as_ref().unwrap() );
                provisioning_salt[32..48].copy_from_slice( &self.random_device );
                let provisioning_salt = &s1( &provisioning_salt )?.into_bytes()[0..];

                let session_key = &device.key_manager.borrow().k1( &provisioning_salt, b"prsk")?.into_bytes()[0..];
                let session_nonce = &device.key_manager.borrow().k1( &provisioning_salt, b"prsn")?.into_bytes()[3..];

                defmt::trace!("** session_key {:x}", session_key);
                defmt::trace!("** session_nonce {:x}", session_nonce);

                let result = aes_ccm_decrypt(&session_key, &session_nonce, &mut data.encrypted, &data.mic);
                match result {
                    Ok(_) => {
                        let provisioning_data = ProvisioningData::parse(&data.encrypted)?;
                        defmt::debug!("** provisioning_data {}", provisioning_data);
                        device.key_manager.borrow_mut().set_provisioning_data(&provisioning_data).await;
                    }
                    Err(_) => {
                        defmt::info!("decryption error!");
                    }
                }
                device.tx_provisioning_pdu(ProvisioningPDU::Complete);
            }
            ProvisioningPDU::Complete => {
                defmt::trace!(">> Complete");
            }
            ProvisioningPDU::Failed(_failed) => {
                defmt::trace!(">> Failed");
            }
        }
        Ok(())
    }

    fn confirmation_device(&self, device: &Device<T, R, S>) -> Result<Confirmation, DeviceError> {
        let salt = self.transcript.confirmation_salt()?;
        let confirmation_key = device.key_manager.borrow().k1(&*salt.into_bytes(), b"prck")?;
        let mut bytes: Vec<u8, 32> = Vec::new();
        bytes.extend_from_slice(&self.random_device).map_err(|_|DeviceError::InsufficientBuffer)?;
        bytes.extend_from_slice(&self.auth_value.as_ref().ok_or(DeviceError::InsufficientBuffer)?.get_bytes()).map_err(|_|DeviceError::InsufficientBuffer)?;
        let confirmation_device = aes_cmac(&confirmation_key.into_bytes(), &bytes)?;

        let mut confirmation = [0; 16];
        for (i, byte) in confirmation_device.into_bytes().iter().enumerate() {
            confirmation[i] = *byte;
        }

        Ok(Confirmation { confirmation })
    }

    fn determine_auth_value(&mut self, device: &Device<T, R, S>, start: &Start) -> Result<AuthValue,DeviceError> {
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

    fn random_physical_oob(&self, device: &Device<T, R, S>, size: u8) -> u32 {
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

    fn random_numeric(&self, device: &Device<T, R, S>, size: u8) -> u32 {
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

    fn random_alphanumeric(&self, device: &Device<T, R, S>, size: u8) -> Result<Vec<u8, 8>, DeviceError> {
        let mut random = Vec::new();
        for _ in 0..size {
            loop {
                let candidate = device.next_random_u8();
                if candidate >= 64 && candidate <= 90 {
                    // Capital ASCII letters A-Z
                    random.push(candidate).map_err(|_|DeviceError::InsufficientBuffer)?;
                } else if candidate >= 48 && candidate <= 57 {
                    // ASCII numbers 0-9
                    random.push(candidate).map_err(|_|DeviceError::InsufficientBuffer)?;
                }
            }
        }
        Ok(random)
    }
}
