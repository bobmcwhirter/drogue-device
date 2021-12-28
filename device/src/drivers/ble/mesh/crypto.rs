
use aes::Aes128;
use cmac::{Cmac, Mac, NewMac};
use cmac::crypto_mac::Output;

const ZERO: [u8; 16] = [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ];

pub fn s1(input: &[u8]) -> Result<Output<Cmac<Aes128>>,()> {
    aes_cmac( &ZERO, input)
}

pub fn aes_cmac(key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, ()> {
    defmt::info!("aes_cmac -A");
    let mut mac = Cmac::<Aes128>::new_from_slice( key ).map_err(|_|())?;
    defmt::info!("aes_cmac -B");
    mac.update(input);
    defmt::info!("aes_cmac -C");
    Ok(mac.finalize())
}

pub fn k1(n: &[u8], salt: &[u8], p: &[u8]) -> Result<Output<Cmac<Aes128>>, ()>{
    defmt::info!("k1 -A");
    let t = aes_cmac(&salt, n).map_err(|_|())?;
    defmt::info!("k1 -B");
    let t = t.into_bytes();
    defmt::info!("k1 -C");
    aes_cmac(&t,p)
}