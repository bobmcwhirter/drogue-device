use crate::drivers::ble::mesh::address::{Address, UnicastAddress};

pub struct NetworkNonce([u8; 13]);

impl NetworkNonce {
    const NONCE_TYPE: u8 = 0x00;

    pub fn new(ctl_ttl: u8, seq: u32, src: [u8; 2], iv_index: u32) -> Self {
        let mut nonce = [0; 13];
        nonce[0] = Self::NONCE_TYPE;
        nonce[1] = ctl_ttl;

        let seq = seq.to_be_bytes();
        nonce[2] = seq[1];
        nonce[3] = seq[2];
        nonce[4] = seq[3];

        nonce[5] = src[0];
        nonce[6] = src[1];

        nonce[7] = 0x00;
        nonce[8] = 0x00;

        let iv_index = iv_index.to_be_bytes();
        nonce[9] = iv_index[0];
        nonce[10] = iv_index[1];
        nonce[11] = iv_index[2];
        nonce[12] = iv_index[3];

        Self(nonce)
    }

    pub fn into_bytes(self) -> [u8; 13] {
        self.0
    }
}

pub struct ApplicationNonce([u8; 13]);

#[derive(Copy, Clone)]
pub struct DeviceNonce([u8; 13]);

impl DeviceNonce {
    const NONCE_TYPE: u8 = 0x02;

    pub fn new(aszmic: bool, seq: u32, src: UnicastAddress, dst: Address, iv_index: u32) -> Self {
        let mut nonce = [0; 13];
        nonce[0] = Self::NONCE_TYPE;
        if aszmic {
            nonce[1] = 0b10000000;
        } else {
            nonce[1] = 0b00000000;
        }

        let seq = seq.to_be_bytes();
        nonce[2] = seq[1];
        nonce[3] = seq[2];
        nonce[4] = seq[3];

        let src = src.as_bytes();
        nonce[5] = src[0];
        nonce[6] = src[1];

        let dst = dst.as_bytes();
        nonce[7] = dst[0];
        nonce[8] = dst[1];

        let iv_index = iv_index.to_be_bytes();
        nonce[9] = iv_index[0];
        nonce[10] = iv_index[1];
        nonce[11] = iv_index[2];
        nonce[12] = iv_index[3];

        Self(nonce)
    }

    pub fn into_bytes(self) -> [u8; 13] {
        self.0
    }
}

pub struct ProxyNonce([u8; 13]);
