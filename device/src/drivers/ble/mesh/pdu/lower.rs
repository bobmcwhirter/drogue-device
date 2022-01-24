use crate::drivers::ble::mesh::app::ApplicationKeyIdentifier;
use crate::drivers::ble::mesh::pdu::ParseError;
use heapless::Vec;

pub enum PDU {
    Access(Access),
    Control(Control),
}

impl PDU {
    pub fn parse(ctl: bool, data: &[u8]) -> Result<Self, ParseError> {
        if data.len() >= 2 {
            let seg = data[0] & 0b10000000 != 0;

            match (ctl, seg) {
                (true, false) => {
                    Ok(PDU::Control(Self::parse_unsegmented_control(data)?))
                }
                (true, true) => {
                    Ok(PDU::Control(Self::parse_segmented_control(data)?))
                }
                (false, false) => {
                    Ok(PDU::Access(Self::parse_unsegmented_access(data)?))
                }
                (false, true) => {
                    Ok(PDU::Access(Self::parse_segmented_access(data)?))
                }
            }
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    fn parse_unsegmented_control(data: &[u8]) -> Result<Control, ParseError> {
        let opcode = Opcode::parse(data[0] & 0b01111111).ok_or(ParseError::InvalidValue)?;
        let parameters = &data[1..];
        Ok(
            Control {
                opcode,
                message: ControlMessage::Unsegmented {
                    parameters: Vec::from_slice(parameters).map_err(|_| ParseError::InsufficientBuffer)?
                }
            }
        )
    }

    fn parse_segmented_control(data: &[u8]) -> Result<Control, ParseError> {
        let opcode = Opcode::parse(data[0] & 0b01111111).ok_or(ParseError::InvalidValue)?;
        let seq_zero = u16::from_be_bytes([ data[1] & 0b01111111, data[2] & 0b11111100 ]) >> 2;
        let seg_o = (u16::from_be_bytes( [ data[2] & 0b00000011, data[3] & 0b11100000 ] ) >> 5) as u8;
        let seg_n = data[3] & 0b00011111;
        let segment_m = &data[4..];
        Ok(
            Control {
                opcode,
                message: ControlMessage::Segmented {
                    seq_zero,
                    seg_o,
                    seg_n,
                    segment_m: Vec::from_slice(segment_m).map_err(|_| ParseError::InsufficientBuffer)?
                }
            }
        )
    }

    fn parse_unsegmented_access(data: &[u8]) -> Result<Access, ParseError> {
        let akf = data[0] & 0b01000000 != 0;
        let aid = data[0] & 0b00111111;
        Ok(
            Access {
                akf,
                aid,
                message: AccessMessage::Unsegmented(
                    Vec::from_slice(&data[1..]).map_err(|_|ParseError::InsufficientBuffer)?
                )
            }
        )
    }

    fn parse_segmented_access(data: &[u8]) -> Result<Access, ParseError> {
        let akf = data[0] & 0b01000000 != 0;
        let aid = data[0] & 0b00111111;
        let szmic = SzMic::parse(data[1] & 0b1000000);
        let seq_zero = u16::from_be_bytes([ data[1] & 0b01111111, data[2] & 0b11111100 ]) >> 2;
        let seg_o = (u16::from_be_bytes( [ data[2] & 0b00000011, data[3] & 0b11100000 ] ) >> 5) as u8;
        let seg_n = data[3] & 0b00011111;
        let segment_m = &data[4..];
        
        Ok(
            Access {
                akf,
                aid,
                message: AccessMessage::Segmented {
                    szmic,
                    seq_zero,
                    seg_o,
                    seg_n,
                    segment_m: Vec::from_slice(&segment_m).map_err(|_|ParseError::InsufficientBuffer)?
                }
            }
        )
    }
}

pub struct Access {
    akf: bool,
    aid: ApplicationKeyIdentifier,
    message: AccessMessage,
}

pub struct Control {
    opcode: Opcode,
    message: ControlMessage,
}

pub enum SzMic {
    Bit32,
    Bit64,
}

impl SzMic {
    pub fn parse(data: u8) -> Self {
        if data != 0 {
            Self::Bit64
        } else {
            Self::Bit32
        }
    }
}

pub enum AccessMessage {
    Unsegmented(Vec<u8, 15>),
    Segmented {
        szmic: SzMic,
        seq_zero: u16,
        seg_o: u8,
        seg_n: u8,
        segment_m: Vec<u8, 12>,
    },
}

pub enum ControlMessage {
    Unsegmented {
        parameters: Vec<u8, 11>,
    },
    Segmented {
        seq_zero: u16,
        seg_o: u8,
        seg_n: u8,
        segment_m: Vec<u8, 8>,
    },
}

pub enum Opcode {
    Reserved = 0x00,
    FriendPoll = 0x01,
    FriendUpdate = 0x02,
    FriendRequest = 0x03,
    FriendOffer = 0x04,
    FriendClear = 0x05,
    FriendClearConfirm = 0x06,
    FriendSubscriptionListAdd = 0x07,
    FriendSubscriptionListRemove = 0x08,
    FriendSubscriptionListConfirm = 0x09,
    Heatbeat = 0x0A,
}

impl Opcode {
    pub fn parse(data: u8) -> Option<Opcode> {
        match data {
            0x01 => Some(Self::FriendPoll),
            0x02 => Some(Self::FriendUpdate),
            0x03 => Some(Self::FriendRequest),
            0x04 => Some(Self::FriendOffer),
            0x05 => Some(Self::FriendClear),
            0x06 => Some(Self::FriendClearConfirm),
            0x07 => Some(Self::FriendSubscriptionListAdd),
            0x08 => Some(Self::FriendSubscriptionListRemove),
            0x09 => Some(Self::FriendSubscriptionListConfirm),
            0x0A => Some(Self::Heatbeat),
            _ => None
        }
    }
}

pub struct SegmentAck {
    seq_zero: u16,
    block_ack: u32,
}
