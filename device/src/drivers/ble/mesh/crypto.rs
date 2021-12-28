use aes::{Aes128, NewBlockCipher};
use ccm::aead::generic_array::GenericArray;
use ccm::aead::{AeadInPlace, Buffer};
use cmac::crypto_mac::Output;
use cmac::{Cmac, Mac, NewMac};
use heapless::Vec;

const ZERO: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

pub fn s1(input: &[u8]) -> Result<Output<Cmac<Aes128>>, ()> {
    aes_cmac(&ZERO, input)
}

pub fn aes_cmac(key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, ()> {
    defmt::info!("aes_cmac -A");
    let mut mac = Cmac::<Aes128>::new_from_slice(key).map_err(|_| ())?;
    defmt::info!("aes_cmac -B");
    mac.update(input);
    defmt::info!("aes_cmac -C");
    Ok(mac.finalize())
}

pub fn k1(n: &[u8], salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, ()> {
    defmt::info!("k1 -A");
    let t = aes_cmac(&salt, n).map_err(|_| ())?;
    defmt::info!("k1 -B");
    let t = t.into_bytes();
    defmt::info!("k1 -C");
    aes_cmac(&t, p)
}

use ccm::aead::NewAead;
use ccm::consts::U13;
use ccm::consts::U8;
use ccm::Ccm;

pub struct CryptoBuffer<const N: usize>(Vec<u8, N>);

impl<const N: usize> CryptoBuffer<N> {
    pub fn from_slice(slice: &[u8]) -> Result<Self,()> {
        let inner = Vec::from_slice(slice)?;
        Ok(Self(inner))
    }
}

impl<const N: usize> AsRef<[u8]> for CryptoBuffer<N> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<const N: usize> AsMut<[u8]> for CryptoBuffer<N> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl<const N: usize> Buffer for CryptoBuffer<N> {
    fn extend_from_slice(&mut self, other: &[u8]) -> ccm::aead::Result<()> {
        self.0
            .extend_from_slice(other)
            .map_err(|_| ccm::aead::Error)
    }

    fn truncate(&mut self, len: usize) {
        self.0.truncate(len)
    }
}

type AesCcm = Ccm<Aes128, U8, U13>;

//pub fn aes_ccm_decrypt<const N: usize>(
pub fn aes_ccm_decrypt(
    key: &[u8],
    nonce: &[u8],
    //data: &mut CryptoBuffer<N>,
    data: &mut [u8],
    mic: &[u8],
) -> Result<(), ()>{
    let key = GenericArray::<u8, <Aes128 as NewBlockCipher>::KeySize>::from_slice(key);
    let ccm = AesCcm::new(&key);
    defmt::info!("MIC: {:x}", mic);
    //ccm.decrypt_in_place(nonce.into(), mic, data).map_err(|_|())
    ccm.decrypt_in_place_detached(nonce.into(), &[], data, mic.into()).map_err(|_|())
}
