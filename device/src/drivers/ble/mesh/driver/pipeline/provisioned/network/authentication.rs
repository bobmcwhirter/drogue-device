use crate::drivers::ble::mesh::address::{Address, UnicastAddress};
use crate::drivers::ble::mesh::configuration_manager::NetworkInfo;
use crate::drivers::ble::mesh::crypto::e;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::network::{AuthenticatedPDU, ObfuscatedAndEncryptedPDU};
use heapless::Vec;
use crate::drivers::ble::mesh::pdu::lower;

pub trait AuthenticationContext {
    fn authenticate(
        &self,
        pdu: &ObfuscatedAndEncryptedPDU,
    ) -> Result<Option<AuthenticationOutput>, DeviceError>;
}

pub struct AuthenticationOutput {
    network: NetworkInfo,
    dst: [u8; 2],
    transport_pdu: Vec<u8, 28>,
}

pub struct Authentication {}

impl Default for Authentication {
    fn default() -> Self {
        Self {

        }
    }
}

impl Authentication {
    pub async fn process_inbound<C: AuthenticationContext>(
        &mut self,
        ctx: &C,
        pdu: ObfuscatedAndEncryptedPDU,
    ) -> Result<Option<AuthenticatedPDU>, DeviceError> {
        if let Some(auth) = ctx.authenticate(&pdu)? {
            let mut privacy_plaintext = [0; 16];

            // 0x0000000000
            privacy_plaintext[0] = 0;
            privacy_plaintext[1] = 0;
            privacy_plaintext[2] = 0;
            privacy_plaintext[3] = 0;
            privacy_plaintext[4] = 0;

            // IV index
            let iv_index_bytes = auth.network.iv_index.to_be_bytes();
            privacy_plaintext[5] = iv_index_bytes[0];
            privacy_plaintext[6] = iv_index_bytes[1];
            privacy_plaintext[7] = iv_index_bytes[2];
            privacy_plaintext[8] = iv_index_bytes[3];

            // Privacy Random
            privacy_plaintext[9] = pdu.encrypted_and_mic[0];
            privacy_plaintext[10] = pdu.encrypted_and_mic[1];
            privacy_plaintext[11] = pdu.encrypted_and_mic[2];
            privacy_plaintext[12] = pdu.encrypted_and_mic[3];
            privacy_plaintext[13] = pdu.encrypted_and_mic[4];
            privacy_plaintext[14] = pdu.encrypted_and_mic[5];
            privacy_plaintext[15] = pdu.encrypted_and_mic[6];

            let pecb =
                e(&auth.network.privacy_key, privacy_plaintext).map_err(|_| DeviceError::InvalidKeyLength)?;

            let unobfuscated = Self::xor(pecb, privacy_plaintext);
            let ctl = (unobfuscated[0] & 0b10000000) != 0;
            let ttl = unobfuscated[0] & 0b01111111;
            let seq = u32::from_be_bytes([0, unobfuscated[1], unobfuscated[2], unobfuscated[3]]);
            let src = UnicastAddress::parse([unobfuscated[4], unobfuscated[5]])
                .map_err(|_| DeviceError::InvalidSrcAddress)?;

            let dst = Address::parse(auth.dst);
            let transport_pdu = lower::PDU::parse(ctl, &*auth.transport_pdu)?;

            Ok(Some(AuthenticatedPDU {
                ivi: pdu.ivi,
                nid: pdu.nid,
                ttl,
                seq,
                src,
                dst,
                transport_pdu,
            }))
        } else {
            Ok(None)
        }
    }

    fn xor(left: [u8; 16], right: [u8; 16]) -> [u8; 16] {
        let mut output = [0; 16];
        for (i, (l, r)) in left.iter().zip(right.iter()).enumerate() {
            output[i] = l ^ r;
        }
        output
    }
}
