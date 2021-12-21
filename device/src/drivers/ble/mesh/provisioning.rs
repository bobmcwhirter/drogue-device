use core::convert::TryInto;
use defmt::Format;
use heapless::Vec;

#[derive(Format)]
pub enum ProvisioningPDU {
    Invite(Invite),
    Capabilities(Capabilities),
    Start {
        algorithm: Algorithm,
        public_key: PublicKey,
        authentication_method: AuthenticationMethod,
        authentication_action: OOBAction,
        authentication_size: OOBSize,
    },
    PublicKey {
        x: [u8; 32],
        y: [u8; 32],
    },
    InputComplete,
    Confirmation {
        confirmation: [u8; 16],
    },
    Random {
        random: [u8; 16],
    },
    Data {
        encrypted: [u8; 25],
        mic: [u8; 8],
    },
    Complete,
    Failed {
        error_code: ErrorCode,
    },
}

#[derive(Format)]
pub struct Invite {
    attention_duration: u8,
}

impl Invite {
    pub fn parse(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 2 && data[0] == 0x00 {
            Ok(Self {
                attention_duration: data[1],
            })
        } else {
            Err(())
        }
    }
}

#[derive(Format, Clone)]
pub struct Capabilities {
    pub number_of_elements: u8,
    pub algorithms: Algorithms,
    pub public_key_type: PublicKeyType,
    pub static_oob_type: StaticOOBType,
    pub output_oob_size: OOBSize,
    pub output_oob_action: OutputOOBActions,
    pub input_oob_size: OOBSize,
    pub input_oob_action: InputOOBActions,
}

impl Capabilities {
    fn parse(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 12 && data[0] == 0x01 {
            let number_of_elements = data[1];
            let algorithms = Algorithms::parse(u16::from_be_bytes([data[2], data[3]]))?;
            let public_key_type = PublicKeyType::parse(data[4])?;
            let static_oob_type = StaticOOBType::parse(data[5])?;
            let output_oob_size = OOBSize::parse(data[6])?;
            let output_oob_action =
                OutputOOBActions::parse(u16::from_be_bytes([data[7], data[8]]))?;
            let input_oob_size = OOBSize::parse(data[9])?;
            let input_oob_action =
                InputOOBActions::parse(u16::from_be_bytes([data[10], data[11]]))?;

            Ok(Self {
                number_of_elements,
                algorithms,
                public_key_type,
                static_oob_type,
                output_oob_size,
                output_oob_action,
                input_oob_size,
                input_oob_action,
            })
        } else {
            Err(())
        }
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {
        xmit.push(0x01);
        xmit.push(self.number_of_elements);
        self.algorithms.emit(xmit);
        self.public_key_type.emit(xmit);
        self.static_oob_type.emit(xmit);
        self.output_oob_size.emit(xmit);
        self.output_oob_action.emit(xmit);
        self.input_oob_size.emit(xmit);
        self.input_oob_action.emit(xmit);
    }
}

impl ProvisioningPDU {
    pub fn parse(data: &[u8]) -> Result<Self, ()> {
        defmt::info!("parsing PDU {:x}", data);
        if data.len() >= 1 {
            match data[0] {
                0x00 => Ok(Self::Invite(Invite::parse(data)?)),
                0x01 => Ok(Self::Capabilities(Capabilities::parse(data)?)),
                0x02 => Self::parse_provisioning_start(&data[1..]),
                0x03 => Self::parse_provisioning_public_key(&data[1..]),
                0x04 => {
                    if data.len() == 1 {
                        Self::parse_provisioning_input_complete()
                    } else {
                        Err(())
                    }
                }
                0x05 => Self::parse_provisioning_confirmation(&data[1..]),
                0x06 => Self::parse_random(&data[1..]),
                0x07 => Self::parse_provisioning_data(&data[1..]),
                0x08 => {
                    if data.len() == 1 {
                        Self::parse_complete()
                    } else {
                        Err(())
                    }
                }
                0x09 => Self::parse_provisioning_failed(&data[1..]),
                _ => Err(()),
            }
        } else {
            Err(())
        }
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {}

    fn parse_provisioning_start(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 5 {
            let algorithm = Algorithm::parse(data[0])?;
            let public_key = PublicKey::parse(data[1])?;
            let authentication_method = AuthenticationMethod::parse(data[2])?;
            let authentication_action = OOBAction::parse(&authentication_method, data[3])?;
            let authentication_size =
                Self::parse_authentication_size(&authentication_method, data[4])?;
            Ok(Self::Start {
                algorithm,
                public_key,
                authentication_method,
                authentication_action,
                authentication_size,
            })
        } else {
            Err(())
        }
    }

    fn parse_authentication_size(method: &AuthenticationMethod, octet: u8) -> Result<OOBSize, ()> {
        match method {
            AuthenticationMethod::NoOOBAuthentication
            | AuthenticationMethod::StaticOOBAuthentication => {
                if octet != 0 {
                    Err(())
                } else {
                    Ok(OOBSize::NotSupported)
                }
            }
            AuthenticationMethod::OutputOOBAuthentication
            | AuthenticationMethod::InputOOBAuthentication => {
                if octet == 0 {
                    Err(())
                } else {
                    OOBSize::parse(octet)
                }
            }
        }
    }

    fn parse_provisioning_public_key(data: &[u8]) -> Result<Self, ()> {
        if data.len() != 64 {
            Err(())
        } else {
            Ok(Self::PublicKey {
                x: data[0..32].try_into().map_err(|_| ())?,
                y: data[32..64].try_into().map_err(|_| ())?,
            })
        }
    }

    fn parse_provisioning_input_complete() -> Result<Self, ()> {
        Ok(Self::InputComplete)
    }

    fn parse_provisioning_confirmation(data: &[u8]) -> Result<Self, ()> {
        if data.len() != 16 {
            Err(())
        } else {
            Ok(Self::Confirmation {
                confirmation: data.try_into().map_err(|_| ())?,
            })
        }
    }

    fn parse_random(data: &[u8]) -> Result<Self, ()> {
        if data.len() != 16 {
            Err(())
        } else {
            Ok(Self::Random {
                random: data.try_into().map_err(|_| ())?,
            })
        }
    }

    fn parse_provisioning_data(data: &[u8]) -> Result<Self, ()> {
        if data.len() != 33 {
            Err(())
        } else {
            Ok(Self::Data {
                encrypted: data[0..25].try_into().map_err(|_| ())?,
                mic: data[25..33].try_into().map_err(|_| ())?,
            })
        }
    }

    fn parse_complete() -> Result<Self, ()> {
        Ok(Self::Complete)
    }

    fn parse_provisioning_failed(data: &[u8]) -> Result<Self, ()> {
        if data.len() != 1 {
            Err(())
        } else {
            Ok(Self::Failed {
                error_code: ErrorCode::parse(data[0])?,
            })
        }
    }
}

#[derive(Format, Clone)]
pub enum Algorithm {
    P256,
}

impl Algorithm {
    pub fn parse(octet: u8) -> Result<Self, ()> {
        if octet == 0x00 {
            Ok(Self::P256)
        } else {
            Err(())
        }
    }
}

#[derive(Format, Clone)]
pub struct Algorithms(Vec<Algorithm, 16>);

impl Algorithms {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, algo: Algorithm) -> Result<(), Algorithm> {
        self.0.push(algo)
    }

    pub fn parse(bits: u16) -> Result<Self, ()> {
        if bits & 0b1111111111111110 != 0 {
            return Err(());
        }

        let mut algos = Algorithms::new();

        if bits & 0b1 == 1 {
            algos.push(Algorithm::P256).map_err(|_| ())?;
        }

        Ok(algos)
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {
        let bits: Option<u16> = self
            .0
            .iter()
            .map(|e| {
                match e {
                    Algorithm::P256 => 0b0000000000000001, // room for growth
                }
            })
            .reduce(|accum, e| accum | e);

        let bits = bits.unwrap_or(0);

        xmit.extend_from_slice(&bits.to_be_bytes());
    }
}

impl Default for Algorithms {
    fn default() -> Self {
        let mut algos = Self::new();
        algos.push(Algorithm::P256);
        algos
    }
}

#[derive(Format, Clone)]
pub struct PublicKeyType {
    pub available: bool,
}

impl Default for PublicKeyType {
    fn default() -> Self {
        Self { available: true }
    }
}

impl PublicKeyType {
    pub fn parse(bits: u8) -> Result<Self, ()> {
        if bits & 0b11111110 != 0 {
            Err(())
        } else {
            Ok(Self {
                available: (bits & 0b1 == 1),
            })
        }
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {
        if self.available {
            xmit.push(0b1);
        } else {
            xmit.push(0b0);
        }
    }
}

#[derive(Format)]
pub enum PublicKey {
    NoPublicKey,
    OOBPublicKey,
}

impl PublicKey {
    pub fn parse(octet: u8) -> Result<Self, ()> {
        match octet {
            0x00 => Ok(Self::NoPublicKey),
            0x01 => Ok(Self::OOBPublicKey),
            _ => Err(()),
        }
    }
}

#[derive(Format, Clone)]
pub struct StaticOOBType {
    pub available: bool,
}

impl StaticOOBType {
    pub fn parse(bits: u8) -> Result<Self, ()> {
        if bits & 0b11111110 != 0 {
            Err(())
        } else {
            Ok(Self {
                available: bits & 0b1 == 1,
            })
        }
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {
        if self.available {
            xmit.push(0b1);
        } else {
            xmit.push(0b0);
        }
    }
}

impl Default for StaticOOBType {
    fn default() -> Self {
        Self { available: false }
    }
}

#[derive(Format, Clone)]
pub enum OOBSize {
    NotSupported,
    MaximumSize(u8 /* 1-8 decimal */),
}

impl OOBSize {
    pub fn parse(octet: u8) -> Result<Self, ()> {
        if octet == 0 {
            Ok(Self::NotSupported)
        } else if octet < 8 {
            Ok(Self::MaximumSize(octet))
        } else {
            Err(())
        }
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {
        match self {
            OOBSize::NotSupported => {
                xmit.push(0);
            }
            OOBSize::MaximumSize(size) => {
                xmit.push(*size);
            }
        }
    }
}

#[derive(Format, Copy, Clone)]
pub enum OutputOOBAction {
    Blink = 0b0000000000000001,
    Beep = 0b0000000000000010,
    Vibrate = 0b0000000000000100,
    OutputNumeric = 0b0000000000001000,
    OutputAlphanumeric = 0b0000000000010000,
}

impl OutputOOBAction {
    pub fn parse(octet: u8) -> Result<Self, ()> {
        match octet {
            0x00 => Ok(Self::Blink),
            0x01 => Ok(Self::Beep),
            0x02 => Ok(Self::Vibrate),
            0x03 => Ok(Self::OutputNumeric),
            0x04 => Ok(Self::OutputAlphanumeric),
            _ => Err(()),
        }
    }
}

#[derive(Format, Clone)]
pub struct OutputOOBActions(Vec<OutputOOBAction, 5>);

impl OutputOOBActions {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, action: OutputOOBAction) -> Result<(), OutputOOBAction> {
        self.0.push(action)
    }

    pub fn parse(bits: u16) -> Result<Self, ()> {
        if bits & 0b1111111111100000 != 0 {
            return Err(());
        }

        let mut actions = OutputOOBActions::new();
        if bits & 0b00000001 == 1 {
            actions.push(OutputOOBAction::Blink).map_err(|_| ())?;
        }

        if bits & 0b00000010 == 1 {
            actions.push(OutputOOBAction::Beep).map_err(|_| ())?;
        }

        if bits & 0b00000100 == 1 {
            actions.push(OutputOOBAction::Vibrate).map_err(|_| ())?;
        }

        if bits & 0b00001000 == 1 {
            actions
                .push(OutputOOBAction::OutputNumeric)
                .map_err(|_| ())?;
        }

        if bits & 0b00010000 == 1 {
            actions
                .push(OutputOOBAction::OutputAlphanumeric)
                .map_err(|_| ())?;
        }

        Ok(actions)
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {
        let bits = self
            .0
            .iter()
            .map(|e| *e as u16)
            .reduce(|accum, e| accum | e);

        let bits = bits.unwrap_or(0);

        xmit.extend_from_slice(&bits.to_be_bytes());
    }
}

impl Default for OutputOOBActions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Format, Copy, Clone)]
pub enum InputOOBAction {
    Push = 0b0000000000000001,
    Twist = 0b0000000000000010,
    InputNumeric = 0b0000000000000100,
    InputAlphanumeric = 0b0000000000001000,
}

impl InputOOBAction {
    pub fn parse(octet: u8) -> Result<Self, ()> {
        match octet {
            0x00 => Ok(Self::Push),
            0x01 => Ok(Self::Twist),
            0x02 => Ok(Self::InputNumeric),
            0x03 => Ok(Self::InputAlphanumeric),
            _ => Err(()),
        }
    }
}

#[derive(Format, Clone)]
pub struct InputOOBActions(Vec<InputOOBAction, 4>);

impl InputOOBActions {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, action: InputOOBAction) -> Result<(), InputOOBAction> {
        self.0.push(action)
    }

    pub fn parse(bits: u16) -> Result<Self, ()> {
        if bits & 0b1111111111110000 != 0 {
            return Err(());
        }

        let mut actions = InputOOBActions::new();
        if bits & 0b00000001 == 1 {
            actions.push(InputOOBAction::Push).map_err(|_| ())?;
        }

        if bits & 0b00000010 == 1 {
            actions.push(InputOOBAction::Twist).map_err(|_| ())?;
        }

        if bits & 0b00000100 == 1 {
            actions.push(InputOOBAction::InputNumeric).map_err(|_| ())?;
        }

        if bits & 0b00001000 == 1 {
            actions
                .push(InputOOBAction::InputAlphanumeric)
                .map_err(|_| ())?;
        }

        Ok(actions)
    }

    pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {
        let bits = self
            .0
            .iter()
            .map(|e| *e as u16)
            .reduce(|accum, e| accum | e);

        let bits = bits.unwrap_or(0);

        xmit.extend_from_slice(&bits.to_be_bytes());
    }
}

impl Default for InputOOBActions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Format, Clone)]
pub enum OOBAction {
    None,
    Output(OutputOOBAction),
    Input(InputOOBAction),
}

impl OOBAction {
    pub fn parse(method: &AuthenticationMethod, octet: u8) -> Result<Self, ()> {
        match method {
            AuthenticationMethod::NoOOBAuthentication
            | AuthenticationMethod::StaticOOBAuthentication => {
                if octet != 0 {
                    Err(())
                } else {
                    Ok(Self::None)
                }
            }
            AuthenticationMethod::OutputOOBAuthentication => {
                Ok(Self::Output(OutputOOBAction::parse(octet)?))
            }
            AuthenticationMethod::InputOOBAuthentication => {
                Ok(Self::Input(InputOOBAction::parse(octet)?))
            }
        }
    }
}

#[derive(Format)]
pub enum AuthenticationMethod {
    NoOOBAuthentication = 0x00,
    StaticOOBAuthentication = 0x01,
    OutputOOBAuthentication = 0x02,
    InputOOBAuthentication = 0x03,
}

impl AuthenticationMethod {
    pub fn parse(octet: u8) -> Result<Self, ()> {
        match octet {
            0x00 => Ok(Self::NoOOBAuthentication),
            0x01 => Ok(Self::StaticOOBAuthentication),
            0x02 => Ok(Self::OutputOOBAuthentication),
            0x03 => Ok(Self::InputOOBAuthentication),
            _ => Err(()),
        }
    }
}

#[derive(Format)]
pub enum ErrorCode {
    Prohibited = 0x00,
    InvalidPDU = 0x01,
    InvalidFormat = 0x02,
    UnexpectedPDU = 0x03,
    ConfirmationFailed = 0x04,
    OutOfResources = 0x05,
    DecryptionFailed = 0x06,
    UnexpectedError = 0x07,
    CannotAssignAddresses = 0x08,
}

impl ErrorCode {
    pub fn parse(octet: u8) -> Result<Self, ()> {
        match octet {
            0x00 => Ok(Self::Prohibited),
            0x01 => Ok(Self::InvalidPDU),
            0x02 => Ok(Self::InvalidFormat),
            0x03 => Ok(Self::UnexpectedPDU),
            0x04 => Ok(Self::ConfirmationFailed),
            0x05 => Ok(Self::OutOfResources),
            0x06 => Ok(Self::DecryptionFailed),
            0x07 => Ok(Self::UnexpectedError),
            0x08 => Ok(Self::CannotAssignAddresses),
            _ => Err(()),
        }
    }
}
