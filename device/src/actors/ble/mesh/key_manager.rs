use crate::actors::ble::mesh::device::{DeviceError, RandomProvider};
use crate::drivers::ble::mesh::crypto::k1;
use crate::drivers::ble::mesh::provisioning::ProvisioningData;
use crate::drivers::ble::mesh::storage::{Payload, Storage};
use aes::Aes128;
use cmac::crypto_mac::{InvalidKeyLength, Output};
use cmac::Cmac;
use core::marker::PhantomData;
use p256::elliptic_curve::ecdh::{diffie_hellman, SharedSecret};
use p256::elliptic_curve::sec1::FromEncodedPoint;
use p256::{AffinePoint, EncodedPoint, NistP256, PublicKey, SecretKey};
use rand_core::{CryptoRng, Error, RngCore};
use crate::actors::ble::mesh::configuration_manager::{Keys, KeyStorage};
use crate::drivers::ble::mesh::transport::Transport;
use core::cell::UnsafeCell;

pub struct KeyManager<R, S>
    where
        R: CryptoRng + RngCore + 'static,
        S: KeyStorage + RandomProvider<R> + 'static,
{
    services: Option<UnsafeCell<*const S>>,
    private_key: Option<SecretKey>,
    peer_public_key: Option<PublicKey>,
    shared_secret: Option<SharedSecret<NistP256>>,
    _marker: PhantomData<(R, S)>,
}

impl<R, S> KeyManager<R, S>
    where
        R: CryptoRng + RngCore + 'static,
        S: KeyStorage + RandomProvider<R> + 'static,
{
    pub fn new() -> Self {
        Self {
            services: None,
            private_key: None,
            peer_public_key: None,
            shared_secret: None,
            _marker: PhantomData,
        }
    }

    fn load_keys(&mut self) -> Result<(), DeviceError> {
        let keys = self.services()?.retrieve();
        self.private_key = keys.private_key().map_err(|_| DeviceError::KeyInitializationError)?;
        self.shared_secret = keys.shared_secret().map_err(|_| DeviceError::KeyInitializationError)?;
        Ok(())
    }

    async fn store_keys(&mut self) -> Result<(), DeviceError> {
        defmt::info!("storing keys");
        let mut keys = Keys::default();
        keys.set_private_key(&self.private_key);
        keys.set_shared_secret(&self.shared_secret);
        self.services()?.store(keys).await.map_err(|_| DeviceError::KeyInitializationError)
    }

    fn set_services(&mut self, services: *const S) {
        self.services.replace(UnsafeCell::new(services));
    }

    fn services(&self) -> Result<&S, DeviceError> {
        match &self.services {
            None => Err(DeviceError::NoServices),
            Some(services) => {
                Ok(unsafe { &**services.get() })
            }
        }
    }

    pub(crate) async fn initialize(
        &mut self,
        services: *const S,
    ) -> Result<(), DeviceError> {
        self.set_services(services);
        self.load_keys()?;

        if let None = self.private_key {
            defmt::info!("** Generating secrets");
            let secret = SecretKey::random(&mut *self.services()?.rng());
            self.private_key.replace(secret);
            defmt::info!("   ...complete");
            self.store_keys().await?
        }
        Ok(())
    }

    pub fn public_key(&self) -> Result<PublicKey, DeviceError> {
        match &self.private_key {
            None => Err(DeviceError::KeyInitializationError),
            Some(private_key) => Ok(private_key.public_key()),
        }
    }

    pub async fn set_peer_public_key(&mut self, pk: PublicKey) -> Result<(), DeviceError> {
        defmt::info!("set_peer_public_key");
        match &self.private_key {
            None => return Err(DeviceError::KeyInitializationError),
            Some(private_key) => {
                self.shared_secret.replace(diffie_hellman(
                    private_key.to_nonzero_scalar(),
                    pk.as_affine(),
                ));
                self.store_keys().await?;
            }
        }
        self.peer_public_key.replace(pk);
        Ok(())
    }

    pub fn set_provisioning_data(&self, data: &ProvisioningData) {}

    pub fn k1(&self, salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        Ok(k1(
            self.shared_secret
                .as_ref()
                .ok_or(DeviceError::NoSharedSecret)?
                .as_bytes(),
            salt,
            p,
        )?)
    }
}
