
use aes::Aes128;
use cmac::{Cmac, Mac, NewMac};
use cmac::crypto_mac::Output;

const ZERO: [u8; 16] = [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ];

pub fn s1(input: &[u8]) -> Result<Output<Cmac<Aes128>>,()> {
    let mut mac = Cmac::<Aes128>::new_from_slice( &ZERO ).map_err(|_|())?;
    mac.update(input);
    Ok(mac.finalize())
}