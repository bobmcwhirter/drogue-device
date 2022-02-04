use crate::drivers::ble::mesh::address::{Address, UnicastAddress};
use crate::drivers::ble::mesh::configuration_manager::{NetworkInfo, NetworkKeyDetails};
use crate::drivers::ble::mesh::crypto::nonce::NetworkNonce;
use crate::drivers::ble::mesh::crypto::{aes_ccm_decrypt_detached, aes_ccm_encrypt_detached, e};
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::lower;
use crate::drivers::ble::mesh::pdu::lower::LowerPDU;
use crate::drivers::ble::mesh::pdu::network::{
    CleartextNetworkPDU, ObfuscatedAndEncryptedNetworkPDU,
};
use heapless::Vec;

pub trait AuthenticationContext {
    fn iv_index(&self) -> Option<u32>;

    fn network_keys(&self, nid: u8) -> Vec<NetworkKeyDetails, 10>;
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
        mut pdu: ObfuscatedAndEncryptedNetworkPDU,
    ) -> Result<Option<CleartextNetworkPDU>, DeviceError> {
        if let Some(iv_index) = ctx.iv_index() {
            let privacy_plaintext = Self::privacy_plaintext(iv_index, &pdu.encrypted_and_mic);

            for network_key in ctx.network_keys(pdu.nid) {
                let pecb = e(&network_key.privacy_key, privacy_plaintext)
                    .map_err(|_| DeviceError::InvalidKeyLength)?;

                let unobfuscated = Self::xor(pecb, pdu.obfuscated);
                let ctl = (unobfuscated[0] & 0b10000000) != 0;

                let seq =
                    u32::from_be_bytes([0, unobfuscated[1], unobfuscated[2], unobfuscated[3]]);

                let nonce = NetworkNonce::new(
                    unobfuscated[0],
                    seq,
                    [unobfuscated[4], unobfuscated[5]],
                    iv_index,
                );

                let encrypted_len = pdu.encrypted_and_mic.len();

                let (payload, mic) = if !ctl {
                    // 32 bit mic
                    pdu.encrypted_and_mic.split_at_mut(encrypted_len - 4)
                } else {
                    // 64 bit mic
                    pdu.encrypted_and_mic.split_at_mut(encrypted_len - 8)
                };

                if let Ok(_) = aes_ccm_decrypt_detached(
                    &network_key.encryption_key,
                    &nonce.into_bytes(),
                    payload,
                    mic,
                ) {
                    let ttl = unobfuscated[0] & 0b01111111;
                    let seq =
                        u32::from_be_bytes([0, unobfuscated[1], unobfuscated[2], unobfuscated[3]]);

                    let src = UnicastAddress::parse([unobfuscated[4], unobfuscated[5]])
                        .map_err(|_| DeviceError::InvalidSrcAddress)?;

                    let dst = Address::parse([payload[0], payload[1]]);

                    let transport_pdu = lower::LowerPDU::parse(ctl, &payload[2..])?;

                    return Ok(Some(CleartextNetworkPDU {
                        network_key,
                        ivi: pdu.ivi,
                        nid: pdu.nid,
                        ttl,
                        seq,
                        src,
                        dst,
                        transport_pdu,
                    }));
                } else {
                    return Err(DeviceError::CryptoError);
                }
            }
        }
        Ok(None)
    }

    pub async fn process_outbound<C: AuthenticationContext>(
        &mut self,
        ctx: &C,
        pdu: CleartextNetworkPDU,
    ) -> Result<Option<ObfuscatedAndEncryptedNetworkPDU>, DeviceError> {
        if let Some(iv_index) = ctx.iv_index() {
            let ctl = match &pdu.transport_pdu {
                LowerPDU::Access(_) => false,
                LowerPDU::Control(_) => true,
            };

            let ctl_ttl = pdu.ttl | (if ctl { 0b10000000 } else { 0 });

            let nonce = NetworkNonce::new(ctl_ttl, pdu.seq, pdu.src.as_bytes(), iv_index);

            if ctl {
                todo!()
            } else {
                let mut mic = [0; 4];

                let mut encrypted_and_mic = Vec::new();
                encrypted_and_mic
                    .extend_from_slice(&pdu.dst.as_bytes())
                    .map_err(|_| DeviceError::InsufficientBuffer)?;
                pdu.transport_pdu.emit(&mut encrypted_and_mic)?;

                aes_ccm_encrypt_detached(
                    &pdu.network_key.encryption_key,
                    &nonce.into_bytes(),
                    &mut encrypted_and_mic,
                    &mut mic,
                )
                .map_err(|_| DeviceError::CryptoError)?;
                encrypted_and_mic
                    .extend_from_slice(&mic)
                    .map_err(|_| DeviceError::InsufficientBuffer)?;

                let privacy_plaintext = Self::privacy_plaintext(iv_index, &encrypted_and_mic);

                let pecb = e(&pdu.network_key.privacy_key, privacy_plaintext)
                    .map_err(|_| DeviceError::InvalidKeyLength)?;

                let mut unobfuscated = [0; 6];
                unobfuscated[0] = ctl_ttl;

                let seq_bytes = pdu.seq.to_be_bytes();
                unobfuscated[1] = seq_bytes[1];
                unobfuscated[2] = seq_bytes[2];
                unobfuscated[3] = seq_bytes[3];

                let src_bytes = pdu.src.as_bytes();
                unobfuscated[4] = src_bytes[0];
                unobfuscated[5] = src_bytes[1];

                let obfuscated = Self::xor(pecb, unobfuscated);

                Ok(Some(ObfuscatedAndEncryptedNetworkPDU {
                    ivi: pdu.ivi,
                    nid: pdu.nid,
                    obfuscated,
                    encrypted_and_mic,
                }))
            }
        } else {
            Err(DeviceError::NotProvisioned)
        }
    }

    fn privacy_plaintext(iv_index: u32, encrypted_and_mic: &[u8]) -> [u8; 16] {
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

        // Privacy Random
        privacy_plaintext[9] = encrypted_and_mic[0];
        privacy_plaintext[10] = encrypted_and_mic[1];
        privacy_plaintext[11] = encrypted_and_mic[2];
        privacy_plaintext[12] = encrypted_and_mic[3];
        privacy_plaintext[13] = encrypted_and_mic[4];
        privacy_plaintext[14] = encrypted_and_mic[5];
        privacy_plaintext[15] = encrypted_and_mic[6];

        privacy_plaintext
    }

    fn xor(pecb: [u8; 16], bytes: [u8; 6]) -> [u8; 6] {
        let mut output = [0; 6];
        for (i, b) in bytes.iter().enumerate() {
            output[i] = pecb[i] ^ *b;
        }
        output
    }
}
