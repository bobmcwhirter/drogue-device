use crate::drivers::ble::mesh::address::{Address, UnicastAddress};
use crate::drivers::ble::mesh::configuration_manager::{NetworkInfo, NetworkKey};
use crate::drivers::ble::mesh::crypto::{aes_ccm_decrypt_detached, e};
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::lower;
use crate::drivers::ble::mesh::pdu::network::{AuthenticatedPDU, ObfuscatedAndEncryptedPDU};
use heapless::Vec;

pub trait AuthenticationContext {
    fn iv_index(&self) -> Option<u32>;

    fn network_keys(&self, nid: u8) -> Vec<NetworkKey, 10>;
}

pub struct AuthenticationOutput {
    network: NetworkInfo,
    dst: [u8; 2],
    transport_pdu: Vec<u8, 28>,
}

pub struct Authentication {}

impl Default for Authentication {
    fn default() -> Self {
        Self {}
    }
}

impl Authentication {
    pub async fn process_inbound<C: AuthenticationContext>(
        &mut self,
        ctx: &C,
        mut pdu: ObfuscatedAndEncryptedPDU,
    ) -> Result<Option<AuthenticatedPDU>, DeviceError> {
        defmt::info!("AUTHN 1");
        if let Some(iv_index) = ctx.iv_index() {
            defmt::info!("AUTHN 2 {:x}", pdu.obfuscated);
            //if let Some(auth) = ctx.authenticate(&pdu)? {
            let mut privacy_plaintext = [0; 16];

            // 0x0000000000
            privacy_plaintext[0] = 0;
            privacy_plaintext[1] = 0;
            privacy_plaintext[2] = 0;
            privacy_plaintext[3] = 0;
            privacy_plaintext[4] = 0;

            // IV index
            let iv_index_bytes = iv_index.to_be_bytes();
            privacy_plaintext[5] = iv_index_bytes[0];
            privacy_plaintext[6] = iv_index_bytes[1];
            privacy_plaintext[7] = iv_index_bytes[2];
            privacy_plaintext[8] = iv_index_bytes[3];
            defmt::info!("AUTHN 3");

            // Privacy Random
            privacy_plaintext[9] = pdu.encrypted_and_mic[0];
            privacy_plaintext[10] = pdu.encrypted_and_mic[1];
            privacy_plaintext[11] = pdu.encrypted_and_mic[2];
            privacy_plaintext[12] = pdu.encrypted_and_mic[3];
            privacy_plaintext[13] = pdu.encrypted_and_mic[4];
            privacy_plaintext[14] = pdu.encrypted_and_mic[5];
            privacy_plaintext[15] = pdu.encrypted_and_mic[6];

            defmt::info!("AUTHN 4");
            for network_key in ctx.network_keys(pdu.nid) {
                defmt::info!("AUTHN 5");
                let pecb = e(&network_key.privacy_key, privacy_plaintext)
                    .map_err(|_| DeviceError::InvalidKeyLength)?;

                defmt::info!("AUTHN 6");
                defmt::info!("obfuscated {:x}", pdu.obfuscated);
                let unobfuscated = Self::xor(pecb, pdu.obfuscated);
                defmt::info!("unobfuscated {:x}", unobfuscated);
                let ctl = (unobfuscated[0] & 0b10000000) != 0;

                let mut nonce = [0;13];
                nonce[0] = 0x00;
                // ctl+ttl
                nonce[1] = unobfuscated[0];

                // seq
                nonce[2] = unobfuscated[1];
                nonce[3] = unobfuscated[2];
                nonce[4] = unobfuscated[3];

                // src
                nonce[5] = unobfuscated[4];
                nonce[6] = unobfuscated[5];

                // padding
                nonce[7] = 0x00;
                nonce[8] = 0x00;

                // IV index
                nonce[9] = iv_index_bytes[0];
                nonce[10] = iv_index_bytes[1];
                nonce[11] = iv_index_bytes[2];
                nonce[12] = iv_index_bytes[3];

                defmt::info!("AUTHN 7");

                let encrypted_len = pdu.encrypted_and_mic.len();

                defmt::info!("AUTHN 8");
                let (payload, mic) = if !ctl {
                    // 32 bit mic
                    pdu.encrypted_and_mic.split_at_mut(encrypted_len - 4)
                } else {
                    // 64 bit mic
                    pdu.encrypted_and_mic.split_at_mut(encrypted_len - 8)
                };
                defmt::info!("AUTHN 9");

                if let Ok(_) =
                    aes_ccm_decrypt_detached(&network_key.encryption_key, &nonce, payload, mic)
                {
                    defmt::info!("AUTHN 10");
                    let ttl = unobfuscated[0] & 0b01111111;
                    let seq =
                        u32::from_be_bytes([0, unobfuscated[1], unobfuscated[2], unobfuscated[3]]);

                    let src = UnicastAddress::parse([unobfuscated[4], unobfuscated[5]])
                        .map_err(|_| DeviceError::InvalidSrcAddress)?;

                    let dst = Address::parse([ payload[0], payload[1] ] );

                    defmt::info!("AUTHN 11");
                    let transport_pdu = lower::PDU::parse(ctl, &payload[2..])?;
                    defmt::info!("AUTHN 12");

                    return Ok(Some(AuthenticatedPDU {
                        ivi: pdu.ivi,
                        nid: pdu.nid,
                        ttl,
                        seq,
                        src,
                        dst,
                        transport_pdu,
                    }))
                } else {
                    defmt::info!("failed to decrypt");
                }
            }
        }
        Ok(None)
    }

    fn xor(pecb: [u8; 16], obfuscated: [u8; 6]) -> [u8; 6] {
        let mut output = [0; 6];
        //for (i, (l, r)) in left.iter().zip(right.iter()).enumerate() {
            //output[i] = l ^ r;
        //}
        for (i, b) in obfuscated.iter().enumerate() {
            output[i] = pecb[i] ^ *b;
        }
        output
    }
}
