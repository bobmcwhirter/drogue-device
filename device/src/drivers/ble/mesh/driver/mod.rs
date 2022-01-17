use cmac::crypto_mac::InvalidKeyLength;
use defmt::Format;
use crate::drivers::ble::mesh::InsufficientBuffer;
use crate::drivers::ble::mesh::provisioning::ParseError;

mod pipeline;
pub mod node;

#[derive(Format)]
pub enum DeviceError {
    CryptoError,
    StorageInitialization,
    KeyInitialization,
    InvalidPacket,
    InsufficientBuffer,
    InvalidLink,
    NoEstablishedLink,
    InvalidKeyLength,
    InvalidTransactionNumber,
    IncompleteTransaction,
    NoSharedSecret,
    ParseError(ParseError),
    TransmitError,
}

impl From<InvalidKeyLength> for DeviceError {
    fn from(_: InvalidKeyLength) -> Self {
        DeviceError::InvalidKeyLength
    }
}

impl From<ParseError> for DeviceError {
    fn from(inner: ParseError) -> Self {
        DeviceError::ParseError(inner)
    }
}

impl From<InsufficientBuffer> for DeviceError {
    fn from(_: InsufficientBuffer) -> Self {
        DeviceError::InsufficientBuffer
    }
}