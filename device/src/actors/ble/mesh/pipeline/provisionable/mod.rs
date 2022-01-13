mod transcript;
mod auth_value;

use core::convert::TryFrom;
use core::future::Future;
use aes::Aes128;
use cmac::Cmac;
use cmac::crypto_mac::Output;
use p256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::provisionable::auth_value::{AuthValue, determine_auth_value};
use crate::actors::ble::mesh::pipeline::provisionable::transcript::Transcript;
use crate::actors::ble::mesh::pipeline::transaction::TransactionContext;
use crate::drivers::ble::mesh::crypto::{aes_ccm_decrypt, aes_cmac, s1};
use crate::drivers::ble::mesh::provisioning::{Capabilities, Confirmation, ProvisioningData, ProvisioningPDU, PublicKey, Random};
use heapless::Vec;
use p256::EncodedPoint;

pub trait ProvisionableContext : TransactionContext {
    fn transmit_provisioning_pdu(&mut self, pdu: ProvisioningPDU);
    fn rng_fill(&mut self, dest: &mut [u8]);


    type SetPeerPublicKeyFuture<'m>: Future<Output = Result<(), DeviceError>>
    where
    Self: 'm;

    fn set_peer_public_key<'m>(&mut self, pk: p256::PublicKey) -> Self::SetPeerPublicKeyFuture<'m>;

    fn public_key(&mut self) -> Result<p256::PublicKey, DeviceError>;

    type SetProvisioningDataFuture<'m>: Future<Output = Result<(), DeviceError>>
    where
    Self: 'm;

    fn set_provisioning_data<'m>(&mut self, data: &ProvisioningData) -> Self::SetProvisioningDataFuture<'m>;

    fn prsk(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError>;
    fn prsn(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError>;
    fn prck(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError>;

    fn rng_u8(&self) -> u8;
    fn rng_u32(&self) -> u32;
}

pub struct Provisionable {
    capabilities: Capabilities,
    transcript: Transcript,
    auth_value: Option<AuthValue>,
    random_device: Option<[u8; 16]>,
    random_provisioner: Option<[u8;16]>,
}

impl Provisionable {

    pub fn new(capabilities: Capabilities) -> Self {
        Self {
            capabilities,
            transcript: Transcript::new(),
            auth_value: None,
            random_device: None,
            random_provisioner: None
        }
    }

    pub async fn process<C: ProvisionableContext>(&mut self, ctx: &mut C, pdu: ProvisioningPDU) -> Result<(), DeviceError> {
        match pdu {
            ProvisioningPDU::Invite(invite) => {
                defmt::trace!(">> Invite");
                self.transcript.add_invite(&invite)?;
                ctx.transmit_provisioning_pdu(ProvisioningPDU::Capabilities(self.capabilities.clone()));
                self.transcript.add_capabilities(&self.capabilities)?
            }
            ProvisioningPDU::Capabilities(_) => {}
            ProvisioningPDU::Start(start) => {
                defmt::trace!(">> Start");
                self.transcript.add_start(&start)?;
                let auth_value = determine_auth_value(ctx, &start)?;
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

                ctx.set_peer_public_key(peer_pk).await;
                let pk = ctx.public_key()?;
                let xy = pk.to_encoded_point(false);
                let x = xy.x().unwrap();
                let y = xy.y().unwrap();
                let pk = PublicKey {
                    x: <[u8; 32]>::try_from(x.as_slice()).map_err(|_| DeviceError::InsufficientBuffer)?,
                    y: <[u8; 32]>::try_from(y.as_slice()).map_err(|_| DeviceError::InsufficientBuffer)?,
                };
                self.transcript.add_pubkey_device(&pk)?;
                defmt::trace!("<< PublicKey");
                defmt::trace!("<<   x = {:x}", &pk.x);
                defmt::trace!("<<   y = {:x}", &pk.y);
                ctx.transmit_provisioning_pdu(ProvisioningPDU::PublicKey(pk));
            }
            ProvisioningPDU::InputComplete => {}
            ProvisioningPDU::Confirmation(confirmation) => {
                defmt::trace!(">> Confirmation");
                defmt::trace!(">>   {}", confirmation);
                let confirmation_device = self.confirmation_device(ctx)?;
                ctx.transmit_provisioning_pdu(ProvisioningPDU::Confirmation(confirmation_device));
            }
            ProvisioningPDU::Random(random) => {
                defmt::trace!(">> Random");
                defmt::trace!(">>   {}", random);
                let mut random_device = [0; 16];
                ctx.rng_fill(&mut random_device);
                ctx.transmit_provisioning_pdu(ProvisioningPDU::Random(Random {
                    random: random_device,
                }));
                self.random_device.replace(random_device);
                self.random_provisioner.replace( random.random );
            }
            ProvisioningPDU::Data(mut data) => {
                defmt::trace!(">> Data");
                defmt::trace!(">>   {}", data);

                let mut provisioning_salt = [0; 48];
                provisioning_salt[0..16].copy_from_slice( &self.transcript.confirmation_salt()?.into_bytes());
                provisioning_salt[16..32].copy_from_slice( self.random_provisioner.as_ref().unwrap() );
                provisioning_salt[32..48].copy_from_slice( self.random_device.as_ref().unwrap() );
                let provisioning_salt = &s1( &provisioning_salt )?.into_bytes()[0..];

                //let session_key = &device.key_manager.borrow().k1( &provisioning_salt, b"prsk")?.into_bytes()[0..];
                let session_key = &ctx.prsk( &provisioning_salt)?.into_bytes()[0..];
                //let session_nonce = &device.key_manager.borrow().k1( &provisioning_salt, b"prsn")?.into_bytes()[3..];
                let session_nonce = &ctx.prsn( &provisioning_salt)?.into_bytes()[3..];

                defmt::trace!("** session_key {:x}", session_key);
                defmt::trace!("** session_nonce {:x}", session_nonce);

                let result = aes_ccm_decrypt(&session_key, &session_nonce, &mut data.encrypted, &data.mic);
                match result {
                    Ok(_) => {
                        let provisioning_data = ProvisioningData::parse(&data.encrypted)?;
                        defmt::debug!("** provisioning_data {}", provisioning_data);
                        ctx.set_provisioning_data(&provisioning_data).await;
                    }
                    Err(_) => {
                        defmt::info!("decryption error!");
                    }
                }
                ctx.transmit_provisioning_pdu(ProvisioningPDU::Complete);

            }
            ProvisioningPDU::Complete => {}
            ProvisioningPDU::Failed(_) => {}
        }
        Ok(())
    }

    fn confirmation_device<C:ProvisionableContext>(&self, ctx: &C) -> Result<Confirmation, DeviceError> {
        let salt = self.transcript.confirmation_salt()?;
        //let confirmation_key = device.key_manager.borrow().k1(&*salt.into_bytes(), b"prck")?;
        let confirmation_key = ctx.prck(&*salt.into_bytes())?;
        let mut bytes: Vec<u8, 32> = Vec::new();
        bytes.extend_from_slice(&self.random_device.unwrap()).map_err(|_|DeviceError::InsufficientBuffer)?;
        bytes.extend_from_slice(&self.auth_value.as_ref().ok_or(DeviceError::InsufficientBuffer)?.get_bytes()).map_err(|_|DeviceError::InsufficientBuffer)?;
        let confirmation_device = aes_cmac(&confirmation_key.into_bytes(), &bytes)?;

        let mut confirmation = [0; 16];
        for (i, byte) in confirmation_device.into_bytes().iter().enumerate() {
            confirmation[i] = *byte;
        }

        Ok(Confirmation { confirmation })
    }

}