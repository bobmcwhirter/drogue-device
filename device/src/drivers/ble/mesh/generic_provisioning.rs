use crate::drivers::ble::mesh::device::Uuid;
use core::convert::TryFrom;
use defmt::Format;
use heapless::Vec;

#[derive(Format)]
pub enum GenericProvisioningPDU {
    TransactionStart {
        seg_n: u8,
        total_len: u16,
        fcs: u8,
        data: Vec<u8, 64>,
    },
    TransactionAck,
    TransactionContinuation {
        segment_index: u8,
        data: Vec<u8, 64>,
    },
    ProvisioningBearerControl(ProvisioningBearerControl),
}

impl GenericProvisioningPDU {
    pub fn parse(data: &[u8]) -> Result<Self, ()> {
        if data.len() >= 1 {
            match data[0] & 0b11 {
                0b00 => Self::parse_transaction_start(data),
                0b01 => Self::parse_transaction_ack(data),
                0b10 => Self::parse_transaction_continuation(data),
                0b11 => Self::ProvisioningBearerControl(ProvisioningBearerControl::parse(data)?),
                _ => Err(()),
            }
        } else {
            Err(())
        }
    }

    fn parse_transaction_start(data: &[u8]) -> Result<Self, ()> {
        if data.len() >= 5 {
            let seg_n = (data[0] & 0b11111100) >> 2;
            let total_len = u16::from_be_bytes([data[1], data[2]]);
            let fcs = data[3];
            let data = data[4..];
            Ok(Self::TransactionStart {
                seg_n,
                total_len,
                fcs,
                data: Vec::from_slice(&data).map_err(|_| ())?,
            })
        } else {
            Err(())
        }
    }

    fn parse_transaction_ack(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 1 {
            if data[0] & 0b11111100 == 0 {
                Ok(Self::TransactionAck)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    fn parse_transaction_continuation(data: &[u8]) -> Result<Self, ()> {
        if data.len() >= 2 {
            let segment_index = (data[0] & 0b11111100) >> 2;
            let data = data[1..];
            Ok(Self::TransactionContinuation {
                segment_index,
                data: Vec::from_slice(&data).map_err(|_| ())?,
            })
        } else {
            Err(())
        }
    }
}

#[derive(Format)]
pub enum ProvisioningBearerControl {
    LinkOpen(Uuid),
    LinkAck,
    LinkClose(Reason),
}

impl ProvisioningBearerControl {
    pub fn parse(data: &[u8]) -> Result<Self, ()> {
        match (data[0] & 0b111111) >> 2 {
            0x00 => Self::parse_link_open(&data[1..]),
            0x01 => Self::parse_link_ack(&data[1..]),
            0x02 => Self::parse_link_close(&data[1..]),
            _ => Err(()),
        }
    }

    fn parse_link_open(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 16 {
            let uuid = Uuid([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
                data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]);
            Ok(Self::LinkOpen(uuid))
        } else {
            Err(())
        }
    }

    fn parse_link_ack(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 0 {
            Ok(Self::LinkAck)
        } else {
            Err(())
        }
    }

    fn parse_link_close(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 1 {
            Ok(Self::LinkClose(Reason::parse(data[0])?))
        } else {
            Err(())
        }
    }
}

pub enum Reason {
    Success = 0x00,
    Timeout = 0x01,
    Fail = 0x02,
}

impl Reason {
    pub fn parse(reason: u8) -> Result<Self, ()> {
        match reason {
            0x00 => Ok(Self::Success),
            0x01 => Ok(Self::Timeout),
            0x02 => Ok(Self::Failse),
            _ => Err(()),
        }
    }
}
