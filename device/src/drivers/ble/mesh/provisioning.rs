use core::convert::TryInto;
use heapless::Vec;

pub enum ProvisioningPDU {
    Invite {
        attention_duration: u8,
    },
    Capabilities {
        number_of_elements: u8,
        algorithms: Algorithms,
        public_key_type: PublicKeyType,
        static_oob_type: StaticOOBType,
        output_oob_size: OOBSize,
        output_oob_action: OutputOOBActions,
        input_oob_size: OOBSize,
        input_oob_action: InputOOBActions,
    },
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

impl ProvisioningPDU {
    pub fn parse(data: &[u8]) -> Result<Self, ()> {
        if data.len() >= 1 {
            match data[0] {
                0x00 => Self::parse_provisioning_invite(&data[1..]),
                0x01 => Self::parse_provisioning_capabilities(&data[1..]),
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

    fn parse_provisioning_invite(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 1 {
            Ok(Self::Invite {
                attention_duration: data[0],
            })
        } else {
            Err(())
        }
    }

    fn parse_provisioning_capabilities(data: &[u8]) -> Result<Self, ()> {
        if data.len() == 11 {
            let number_of_elements = data[0];
            let algorithms = Self::parse_algorithms(u16::from_be_bytes([data[1], data[2]]))?;
            let public_key_type = Self::parse_public_key_types(data[3])?;
            let static_oob_type = Self::parse_static_oob_type(data[4])?;
            let output_oob_size = OOBSize::parse(data[5])?;
            let output_oob_action =
                Self::parse_output_oob_action(u16::from_be_bytes([data[6], data[7]]))?;
            let input_oob_size = OOBSize::parse(data[8])?;
            let input_oob_action =
                Self::parse_input_oob_action(u16::from_be_bytes([data[9], data[10]]))?;

            Ok(Self::Capabilities {
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

    fn parse_algorithms(bits: u16) -> Result<Algorithms, ()> {
        if bits & 0b1111111111111110 != 0 {
            return Err(());
        }

        let mut algos = Algorithms::new();

        if bits & 0b1 == 1 {
            algos.push(Algorithm::P256)
        }

        Ok(algos)
    }

    fn parse_public_key_types(bits: u8) -> Result<PublicKeyType, ()> {
        if bits & 0b11111110 != 0 {
            return Err(());
        }

        Ok(PublicKeyType {
            available: (bits & 0b1 == 1),
        })
    }

    fn parse_static_oob_type(bits: u8) -> Result<StaticOOBType, ()> {
        if bits & 0b11111110 != 0 {
            Err(())
        } else {
            Ok(StaticOOBType {
                available: bits & 0b1 == 1,
            })
        }
    }

    fn parse_output_oob_action(bits: u16) -> Result<OutputOOBActions, ()> {
        if bits & 0b1111111111100000 != 0 {
            return Err(());
        }

        let mut actions = OutputOOBActions::new();
        if bits & 0b00000001 == 1 {
            actions.push(OutputOOBAction::Blink)
        }

        if bits & 0b00000010 == 1 {
            actions.push(OutputOOBAction::Beep)
        }

        if bits & 0b00000100 == 1 {
            actions.push(OutputOOBAction::Vibrate)
        }

        if bits & 0b00001000 == 1 {
            actions.push(OutputOOBAction::OutputNumeric)
        }

        if bits & 0b00010000 == 1 {
            actions.push(OutputOOBAction::OutputAlphanumeric)
        }

        Ok(actions)
    }

    fn parse_input_oob_actions(bits: u16) -> Result<InputOOBActions, ()> {
        if bits & 0b1111111111110000 != 0 {
            return Err(());
        }

        let mut actions = InputOOBActions::new();
        if bits & 0b00000001 == 1 {
            actions.push(InputOOBAction::Push)
        }

        if bits & 0b00000010 == 1 {
            actions.push(InputOOBAction::Twist)
        }

        if bits & 0b00000100 == 1 {
            actions.push(InputOOBAction::InputNumeric)
        }

        if bits & 0b00001000 == 1 {
            actions.push(InputOOBAction::InputAlphanumeric)
        }

        Ok(actions)
    }

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
                    OOBSize::NotSupported
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
                x: data[0..32].try_into()?,
                y: data[32..64].try_into()?,
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
                confirmation: data.try_into()?,
            })
        }
    }

    fn parse_random(data: &[u8]) -> Result<Self, ()> {
        if data.len() != 16 {
            Err(())
        } else {
            Ok(Self::Random {
                random: data.try_into()?,
            })
        }
    }

    fn parse_provisioning_data(data: &[u8]) -> Result<Self, ()> {
        if data.len() != 33 {
            Err(())
        } else {
            Ok(Self::Data {
                encrypted: data[0..25].try_into()?,
                mic: data[25..33].try_into()?,
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

pub type Algorithms = Vec<Algorithm, 16>;

pub struct PublicKeyType {
    pub available: bool,
}

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

pub struct StaticOOBType {
    pub available: bool,
}

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
}

pub enum OutputOOBAction {
    Blink,
    Beep,
    Vibrate,
    OutputNumeric,
    OutputAlphanumeric,
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

pub type OutputOOBActions = Vec<OutputOOBAction, 5>;

pub enum InputOOBAction {
    Push,
    Twist,
    InputNumeric,
    InputAlphanumeric,
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

pub type InputOOBActions = Vec<InputOOBAction, 4>;

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
