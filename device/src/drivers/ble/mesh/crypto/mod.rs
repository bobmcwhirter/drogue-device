use aes::{Aes128, BlockEncrypt, NewBlockCipher};
use ccm::aead::generic_array::GenericArray;
use ccm::aead::{AeadMutInPlace, Buffer, NewAead};
use ccm::aead::{AeadInPlace, Error};
use ccm::consts::U13;
use ccm::consts::U4;
use ccm::consts::U8;
use ccm::Ccm;
use cmac::crypto_mac::{InvalidKeyLength, Output};
use cmac::{Cmac, Mac, NewMac};
use core::convert::TryInto;
use core::iter::FromIterator;
use aes::cipher::Block;
use heapless::Vec;

pub mod nonce;

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

pub fn k2(n: &[u8], p: &[u8]) -> Result<(u8, [u8; 16], [u8; 16]), InvalidKeyLength> {
    let salt = s1(b"smk2")?;
    let t = &aes_cmac(&salt.into_bytes(), n)?.into_bytes();

    let mut input: Vec<u8, 64> = Vec::new();
    input.extend_from_slice(p).map_err(|_| InvalidKeyLength)?;
    input.push(0x01).map_err(|_| InvalidKeyLength)?;
    let t1 = &aes_cmac(t, &input)?.into_bytes();

    let nid = t1[15] & 0x7F;
    defmt::info!("NID {:x}", nid);

    input.truncate(0);
    input.extend_from_slice(&t1).map_err(|_| InvalidKeyLength)?;
    input.extend_from_slice(p).map_err(|_| InvalidKeyLength)?;
    input.push(0x02).map_err(|_| InvalidKeyLength)?;

    let t2 = aes_cmac(t, &input)?.into_bytes();

    let encryption_key = t2;

    input.truncate(0);
    input.extend_from_slice(&t2).map_err(|_| InvalidKeyLength)?;
    input.extend_from_slice(p).map_err(|_| InvalidKeyLength)?;
    input.push(0x03).map_err(|_| InvalidKeyLength)?;

    let t3 = aes_cmac(t, &input)?.into_bytes();
    let privacy_key = t3;

    Ok((
        nid,
        encryption_key.try_into().map_err(|_| InvalidKeyLength)?,
        privacy_key.try_into().map_err(|_| InvalidKeyLength)?,
    ))
}

pub fn e(key: &[u8], mut data: [u8;16])  -> Result<[u8;16], InvalidKeyLength>{
    let key = GenericArray::<u8, <Aes128 as NewBlockCipher>::KeySize>::from_slice(key);
    let mut cipher = Aes128::new_from_slice(key).map_err(|_|InvalidKeyLength)?;

    let mut cipher_block = Block::<Aes128>::from_mut_slice(&mut data);
    cipher.encrypt_block( &mut cipher_block);
    Ok(data)
}

type AesCcm_32bitMac = Ccm<Aes128, U4, U13>;
type AesCcm_64bitMac = Ccm<Aes128, U8, U13>;

pub fn aes_ccm_decrypt_detached(key: &[u8], nonce: &[u8], data: &mut [u8], mic: &[u8]) -> Result<(), Error> {
    defmt::info!("decrypt a");
    let key = GenericArray::<u8, <Aes128 as NewBlockCipher>::KeySize>::from_slice(key);
    defmt::info!("decrypt b");
    match mic.len() {
        4 => {
            defmt::info!("decrypt c");
            let ccm = AesCcm_32bitMac::new(&key);
            defmt::info!("decrypt d {:x} {:x}", data, mic);
            ccm.decrypt_in_place_detached(nonce.into(), &[], data, mic.into())
        }
        8 => {
            defmt::info!("decrypt e");
            let ccm = AesCcm_64bitMac::new(&key);
            defmt::info!("decrypt f {:x} {:x}", data, mic);
            ccm.decrypt_in_place_detached(nonce.into(), &[], data, mic.into())
        }
        _ => {
            Err(Error)
        }
    }
}

