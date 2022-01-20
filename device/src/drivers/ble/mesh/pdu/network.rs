use core::convert::TryInto;
use crate::drivers::ble::mesh::address::{Address, UnicastAddress};
use crate::drivers::ble::mesh::pdu::{lower, ParseError};
use crate::drivers::ble::mesh::MESH_MESSAGE;
use heapless::Vec;

pub enum NetMic {
    Access([u8;4]),
    Control([u8;8]),
}

pub struct PDU {
    ivi: u8, /* 1 bit */
    nid: u8, /* 7 bits */
    // ctl: bool /* 1 bit */
    ttl: u8,  /* 7 bits */
    seq: u32, /* 24 bits */
    src: UnicastAddress,
    dst: Address,
    transport_pdu: lower::PDU,
    net_mic: NetMic,
}

impl PDU {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() >= 11 {
            if data[1] != MESH_MESSAGE {
                Err(ParseError::InvalidPDUFormat)
            } else {
                let ivi_nid = data[2];
                let ivi = ivi_nid & 0b10000000 >> 7;
                let nid = ivi_nid & 0b01111111;
                let ctl_ttl = data[3];
                let ctl = ctl_ttl & 0b1000000 != 0;
                let ttl = ctl_ttl & 0b01111111;
                let seq = u32::from_be_bytes([0, data[4], data[5], data[6]]);
                let src = UnicastAddress::parse([data[7], data[8]])
                    .map_err(|_| ParseError::InvalidValue)?;
                let dst = Address::parse([data[9], data[10]]);
                let transport_pdu = lower::PDU::parse(ctl, &data[11..])?;
                let net_mic = if ctl {
                    if data.len() < 13+8 {
                        return Err(ParseError::InvalidLength)
                    }
                    NetMic::Control(data[12..=12+8].try_into().map_err(|_|ParseError::InvalidLength)?)
                } else {
                    if data.len() < 13+4 {
                        return Err(ParseError::InvalidLength)
                    }
                    NetMic::Control(data[12..=12+4].try_into().map_err(|_|ParseError::InvalidLength)?)
                };
                Ok(Self {
                    ivi,
                    nid,
                    ttl,
                    seq,
                    src,
                    dst,
                    transport_pdu,
                    net_mic,
                })
            }
        } else {
            Err(ParseError::InvalidPDUFormat)
        }
    }
}
