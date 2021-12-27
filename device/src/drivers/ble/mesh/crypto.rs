
use aes::Aes128;
use cmac::{Cmac, Mac, NewMac};
use cmac::crypto_mac::Output;

const ZERO: [u8; 16] = [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ];

pub fn s1(input: &[u8]) -> Result<Output<Cmac<Aes128>>,()> {
    aes_cmac( &ZERO, input)
}

pub fn aes_cmac(key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, ()> {
    let mut mac = Cmac::<Aes128>::new_from_slice( key ).map_err(|_|())?;
    mac.update(input);
    Ok(mac.finalize())
}

pub fn k1(n: &[u8], salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, ()>{
    let t = aes_cmac(&salt, n).map_err(|_|())?;
    let t = t.into_bytes();
    aes_cmac(&t,p)
}