use heapless::Vec;

pub enum PDU {
    Control(Control),
    Access(Access),
}

pub struct Control {
    data: Vec<u8,256>,
}

pub enum TransMIC {
    Bit32([u8;4]),
    Bit64([u8;8]),
}

pub struct Access {
    payload: Vec<u8, 380>,
    transmit: TransMIC,
}
