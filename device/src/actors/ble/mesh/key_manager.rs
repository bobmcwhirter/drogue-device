use crate::actors::ble::mesh::device::DeviceError;
use crate::drivers::ble::mesh::crypto::k1;
use crate::drivers::ble::mesh::key_storage::{KeyStorage, Keys};
use crate::drivers::ble::mesh::provisioning::ProvisioningData;
use aes::Aes128;
use cmac::crypto_mac::{InvalidKeyLength, Output};
use cmac::Cmac;
use core::cell::RefCell;
use p256::elliptic_curve::ecdh::{diffie_hellman, SharedSecret};
use p256::elliptic_curve::sec1::FromEncodedPoint;
use p256::{AffinePoint, EncodedPoint, NistP256, PublicKey, SecretKey};
use rand_core::{CryptoRng, Error, RngCore};

pub struct KeyManager<S>
where
    S: KeyStorage,
{
    storage: S,
    private_key: Option<SecretKey>,
    peer_public_key: RefCell<Option<PublicKey>>,
    shared_secret: RefCell<Option<SharedSecret<NistP256>>>,
}

impl<S> KeyManager<S>
where
    S: KeyStorage,
{
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            private_key: None,
            peer_public_key: RefCell::new(None),
            shared_secret: RefCell::new(None),
        }
    }

    async fn load_keys(&mut self) -> Result<bool, DeviceError> {
        if let Ok(Some(data)) = self.storage.retrieve().await {
            defmt::info!("** Loading secrets");

            if data.payload[0] == 1 {
                let private_key = &data.payload[1..1 + 32];
                defmt::info!("** private key {} {:x}", private_key.len(), private_key);
                let private_key = SecretKey::from_be_bytes(private_key)
                    .map_err(|_| DeviceError::KeyInitializationError)?;
                self.private_key.replace(private_key);
            }
            defmt::info!("** Loading secrets 1");

            if data.payload[33] == 1 {
                let affine = AffinePoint::from_encoded_point(
                    &EncodedPoint::from_bytes(&data.payload[34..34 + 16])
                        .map_err(|_| DeviceError::KeyInitializationError)?,
                );
                defmt::info!("** Loading secrets 2");
                if !bool::from(affine.is_some()) {
                    defmt::info!("** Loading secrets 3");
                    return Err(DeviceError::KeyInitializationError);
                }
                defmt::info!("** Loading secrets 4");
                let shared_secret = SharedSecret::from(&affine.unwrap());
                defmt::info!("** Loading secrets 5");
                self.shared_secret.borrow_mut().replace(shared_secret);
                defmt::info!("** Loading secrets 6");
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn store_keys(&mut self) -> Result<(), DeviceError> {
        let mut payload = [0; 512];
        if let Some(private_key) = &self.private_key {
            payload[0] = 1;
            let private_key = private_key.to_nonzero_scalar().to_bytes();
            defmt::info!("private key {} {:x}", private_key.len(), &*private_key);
            for (i, byte) in private_key.iter().enumerate() {
                payload[i + 1] = *byte;
            }
        }
        if let Some(shared_secret) = &*self.shared_secret.borrow() {
            payload[17] = 1;
            let shared_secret = shared_secret.as_bytes();
            for (i, byte) in shared_secret.iter().enumerate() {
                payload[i + 18] = *byte;
            }
        }

        let keys = Keys { payload };
        self.storage
            .store(&keys)
            .await
            .map_err(|_| DeviceError::KeyInitializationError)
    }

    pub(crate) async fn initialize<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
    ) -> Result<(), DeviceError> {
        
        self.load_keys().await?;

        if let None = self.private_key {
            defmt::info!("** Generating secrets");
            let secret = SecretKey::random(rng);
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

    pub fn set_peer_public_key(&self, pk: PublicKey) -> Result<(), DeviceError> {
        match &self.private_key {
            None => return Err(DeviceError::KeyInitializationError),
            Some(private_key) => {
                self.shared_secret.borrow_mut().replace(diffie_hellman(
                    private_key.to_nonzero_scalar(),
                    pk.as_affine(),
                ));
            }
        }
        self.peer_public_key.borrow_mut().replace(pk);
        Ok(())
    }

    pub fn set_provisioning_data(&self, data: &ProvisioningData) {}

    pub fn k1(&self, salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        Ok(k1(
            self.shared_secret
                .borrow()
                .as_ref()
                .ok_or(DeviceError::NoSharedSecret)?
                .as_bytes(),
            salt,
            p,
        )?)
    }
}
