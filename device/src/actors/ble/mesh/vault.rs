use core::convert::TryInto;
use core::future::Future;

use aes::{Aes128, NewBlockCipher};
use aes::cipher::generic_array::GenericArray;
use ccm::aead::{AeadInPlace, Buffer};
use ccm::Ccm;
use ccm::consts::U13;
use ccm::aead::NewAead;
use ccm::consts::U8;
use cmac::{Cmac, Mac, NewMac};
use cmac::crypto_mac::{InvalidKeyLength, Output};
use heapless::Vec;
use p256::PublicKey;

use crate::actors::ble::mesh::device::DeviceError;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::provisioning::ProvisioningData;

type AesCcm = Ccm<Aes128, U8, U13>;

pub trait Vault {
    fn uuid(&self) -> Uuid;

    type SetPeerPublicKeyFuture<'m>: Future<Output = Result<(), DeviceError>>
    where
    Self: 'm;

    fn set_peer_public_key<'m>(&mut self, pk: PublicKey) -> Self::SetPeerPublicKeyFuture<'m>;

    fn public_key(&self) -> Result<PublicKey, DeviceError>;

    fn aes_cmac(&self, key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        let mut mac = Cmac::<Aes128>::new_from_slice(key)?;
        mac.update(input);
        Ok(mac.finalize())
    }

    const ZERO: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    fn s1(&self, input: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.aes_cmac(&Self::ZERO, input)
    }

    fn k1(&self, n: &[u8], salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        let t = self.aes_cmac(&salt, n)?;
        let t = t.into_bytes();
        self.aes_cmac(&t, p)
    }

    fn n_k1(&self, salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError>;

    fn prsk(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.n_k1(salt, b"prsk")
    }

    fn prsn(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.n_k1(salt, b"prsn")
    }

    fn prck(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.n_k1(salt, b"prck")
    }

    fn k2(&self, n: &[u8], p: &[u8]) -> Result<(u8, [u8; 16], [u8; 16]), DeviceError> {
        let salt = self.s1(b"smk2")?;
        let t = &self.aes_cmac(&salt.into_bytes(), n)?.into_bytes();

        let mut input: Vec<u8, 64> = Vec::new();
        input.extend_from_slice(p).map_err(|_| DeviceError::InvalidKeyLength)?;
        input.push(0x01);
        let t1 = &self.aes_cmac(t, &input)?.into_bytes();

        let nid = t1[15] & 0x7F;
        defmt::info!("NID {:x}", nid);

        input.truncate(0);
        input.extend_from_slice(&t1).map_err(|_| DeviceError::InvalidKeyLength)?;
        input.extend_from_slice(p).map_err(|_| DeviceError::InvalidKeyLength)?;
        input.push(0x02);

        let t2 = self.aes_cmac(t, &input)?.into_bytes();

        let encryption_key = t2;

        input.truncate(0);
        input.extend_from_slice(&t2).map_err(|_| DeviceError::InvalidKeyLength)?;
        input.extend_from_slice(p).map_err(|_| DeviceError::InvalidKeyLength)?;
        input.push(0x03);

        let t3 = self.aes_cmac(t, &input)?.into_bytes();
        let privacy_key = t3;

        Ok((
            nid,
            encryption_key.try_into().map_err(|_| DeviceError::InvalidKeyLength)?,
            privacy_key.try_into().map_err(|_| DeviceError::InvalidKeyLength)?,
        ))
    }

    fn aes_ccm_decrypt(
        &self,
        key: &[u8],
        nonce: &[u8],
        data: &mut [u8],
        mic: &[u8],
    ) -> Result<(), DeviceError> {
        let key = GenericArray::<u8, <Aes128 as NewBlockCipher>::KeySize>::from_slice(key);
        let ccm = AesCcm::new(&key);
        ccm.decrypt_in_place_detached(nonce.into(), &[], data, mic.into()).map_err(|_|DeviceError::CryptoError)
    }

    type SetProvisioningDataFuture<'m>: Future<Output = Result<(), DeviceError>>
    where
    Self: 'm;

    fn set_provisioning_data<'m>(
        &mut self,
        data: &'m ProvisioningData,
    ) -> Self::SetProvisioningDataFuture<'m>;
}