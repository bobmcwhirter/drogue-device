use core::convert::TryInto;
use core::cell::RefCell;
use crate::drivers::ble::mesh::storage::{Payload, Storage};
use futures::future::Future;
use p256::ecdh::SharedSecret;
use p256::elliptic_curve::sec1::FromEncodedPoint;
use p256::{AffinePoint, EncodedPoint, SecretKey};
use postcard::{from_bytes, to_slice};
use serde::{Deserialize, Serialize};
use defmt::Format;

#[derive(Serialize, Deserialize, Copy, Clone, Default, Format)]
pub struct Configuration {
    uuid: Option<[u8; 16]>,
    keys: Keys,
}

#[derive(Serialize, Deserialize, Copy, Clone, Default, Format)]
pub struct Keys {
    random: Option<[u8; 16]>,
    private_key: Option<[u8; 32]>,
    shared_secret: Option<[u8; 32]>,
}

impl Keys {
    pub(crate) fn private_key(&self) -> Result<Option<SecretKey>, ()> {
        match self.private_key {
            None => Ok(None),
            Some(private_key) => Ok(Some(
                SecretKey::from_be_bytes(&private_key).map_err(|_| ())?,
            )),
        }
    }

    pub(crate) fn set_private_key(&mut self, private_key: &Option<SecretKey>) -> Result<(),()> {
        match private_key {
            None => {
                self.private_key.take();
            },
            Some(private_key) => {
                self.private_key.replace( private_key.to_nonzero_scalar().to_bytes().try_into().map_err(|_|())? );
            }
        }
        Ok(())
    }

    pub(crate) fn shared_secret(&self) -> Result<Option<SharedSecret>, ()> {
        match self.shared_secret {
            None => Ok(None),
            Some(shared_secret) => {
                let affine = AffinePoint::from_encoded_point(
                    &EncodedPoint::from_bytes(&shared_secret).map_err(|_| ())?,
                );
                if !bool::from(affine.is_some()) {
                    return Err(());
                }
                Ok(Some(SharedSecret::from(&affine.unwrap())))
            }
        }
    }

    pub(crate) fn set_shared_secret(&mut self, shared_secret: &Option<SharedSecret>) -> Result<(),()> {
        match shared_secret {
            None => {
                self.shared_secret.take();
            }
            Some(shared_secret) => {
                self.shared_secret.replace(shared_secret.as_bytes()[0..].try_into().map_err(|_| ())?);
            }
        }
        Ok(())
    }
}

pub trait KeyStorage {
    type StoreFuture<'m>: Future<Output = Result<(), ()>>
    where
        Self: 'm;

    fn store<'m>(&'m self, keys: Keys) -> Self::StoreFuture<'m>;

    fn retrieve<'m>(&'m self) -> Keys;
}

pub struct ConfigurationManager<S: Storage> {
    storage: RefCell<S>,
    config: RefCell<Configuration>,
}

impl<S: Storage> KeyStorage for ConfigurationManager<S> {
    type StoreFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), ()>>;

    fn store<'m>(&'m self, keys: Keys) -> Self::StoreFuture<'m> {
        let mut update = self.config.borrow().clone();
        update.keys = keys;
        async move { self.store(&update).await }
    }

    fn retrieve<'m>(&'m self) -> Keys {
        self.config.borrow().keys
    }
}

impl<S: Storage> ConfigurationManager<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage: RefCell::new(storage),
            config: RefCell::new(Default::default()),
        }
    }

    pub(crate) async fn initialize(&mut self) -> Result<(), ()> {
        let payload = self.storage.borrow_mut().retrieve().await?;
        match payload {
            None => Err(()),
            Some(payload) => {
                self.config = from_bytes(&payload.payload).map_err(|_| ())?;
                Ok(())
            }
        }
    }

    fn retrieve(&self) -> Configuration {
        *self.config.borrow()
    }

    async fn store(&self, config: &Configuration) -> Result<(), ()> {
        defmt::info!("storing {}", config);
        let mut payload = [0; 512];
        to_slice(config, &mut payload);
        let payload = Payload { payload };
        self.storage.borrow_mut().store(&payload).await?;
        self.config.replace(*config);
        Ok(())
    }
}
