use defmt::Format;

#[derive(Copy, Clone, Format)]
pub enum Address {
    Unassigned,
    Unicast(UnicastAddress),
    Virtual(VirtualAddress),
    Group(GroupAddress),
}

#[derive(Copy, Clone, Format)]
pub struct InvalidAddress;

#[derive(Copy, Clone, Format)]
pub struct UnicastAddress([u8; 2]);

#[derive(Copy, Clone, Format)]
pub struct VirtualAddress([u8; 2]);

#[derive(Copy, Clone, Format)]
pub struct GroupAddress([u8; 2]);

impl UnicastAddress {
    pub fn is_unicast_address(data: &[u8; 2]) -> bool {
        data[0] & 0b10000000 == 0
    }

    pub fn parse(data: [u8; 2]) -> Result<Self, InvalidAddress> {
        if Self::is_unicast_address(&data) {
            Ok(UnicastAddress(data))
        } else {
            Err(InvalidAddress)
        }
    }
}

impl VirtualAddress {
    pub fn is_virtual_address(data: &[u8;2]) -> bool {
        data[0] & 0b11000000 == 0b10000000
    }

    pub fn parse(data: [u8; 2]) -> Result<Self, InvalidAddress> {
        if Self::is_virtual_address(&data) {
            Ok(VirtualAddress(data))
        } else {
            Err(InvalidAddress)
        }
    }
}

impl GroupAddress {
    pub fn is_group_address(data: &[u8; 2]) -> bool {
        data[0] & 0b11000000 == 0b11000000
    }

    pub fn parse(data: [u8; 2]) -> Result<Self, InvalidAddress> {
        if Self::is_group_address(&data) {
            Ok(GroupAddress(data))
        } else {
            Err(InvalidAddress)
        }
    }
}

impl Address {
    pub fn parse(data: [u8; 2]) -> Self {
        if data[0] == 0 && data[1] == 0 {
            Self::Unassigned
        } else if UnicastAddress::is_unicast_address(&data) {
            Self::Unicast(UnicastAddress([data[0], data[1]]))
        } else if GroupAddress::is_group_address(&data) {
            Self::Group(GroupAddress([data[0], data[1]]))
        } else {
            Self::Virtual(VirtualAddress([data[0], data[1]]))
        }
    }
}
