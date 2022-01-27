use defmt::Format;
use heapless::Vec;

#[derive(Format)]
pub enum PDU {
    Control(Control),
    Access(Access),
}

#[derive(Format)]
pub struct Control {
    data: Vec<u8,256>,
}

#[derive(Format)]
pub enum TransMIC {
    Bit32([u8;4]),
    Bit64([u8;8]),
}

#[derive(Format)]
pub struct Access {
    pub(crate) payload: Vec<u8, 380>,
    pub(crate) trans_mic: TransMIC,
}
