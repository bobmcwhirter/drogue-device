use aes::{Aes128, NewBlockCipher};
use ccm::aead::{AeadInPlace, Buffer, Error as CryptoError};
use ccm::aead::generic_array::GenericArray;
use ccm::aead::NewAead;
use ccm::Ccm;
use ccm::consts::U13;
use ccm::consts::U8;
use cmac::{Cmac, Mac, NewMac};
use cmac::crypto_mac::{InvalidKeyLength, Output};
use heapless::Vec;

use crate::drivers::ble::mesh::InsufficientBuffer;

const ZERO: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

pub fn s1(input: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    aes_cmac(&ZERO, input)
}

pub fn aes_cmac(key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    let mut mac = Cmac::<Aes128>::new_from_slice(key)?;
    mac.update(input);
    Ok(mac.finalize())
}

pub fn k1(n: &[u8], salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, InvalidKeyLength> {
    let t = aes_cmac(&salt, n)?;
    let t = t.into_bytes();
    aes_cmac(&t, p)
}

type AesCcm = Ccm<Aes128, U8, U13>;

pub fn aes_ccm_decrypt(
    key: &[u8],
    nonce: &[u8],
    data: &mut [u8],
    mic: &[u8],
) -> Result<(), CryptoError>{
    let key = GenericArray::<u8, <Aes128 as NewBlockCipher>::KeySize>::from_slice(key);
    let ccm = AesCcm::new(&key);
    ccm.decrypt_in_place_detached(nonce.into(), &[], data, mic.into())
}
