use defmt::{Format, Formatter};
use crate::drivers::ble::mesh::pdu::ParseError;

#[derive(Format)]
pub enum AccessMessage {
    Config(Config),
    Health(Health),
}

impl AccessMessage {
    pub fn opcode(&self) -> Opcode {
        match self {
            AccessMessage::Config(inner) => inner.opcode(),
            AccessMessage::Health(inner) => inner.opcode(),
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let (opcode, parameters) = Opcode::split(data).ok_or(ParseError::InvalidPDUFormat)?;
        defmt::info!("OPCODE {}", opcode);
        match opcode {
            CONFIG_NODE_RESET => Ok(Self::Config(Config::NodeReset(NodeReset::parse_reset(
                parameters,
            )?))),
            CONFIG_NODE_RESET_STATUS => Ok(Self::Config(Config::NodeReset(
                NodeReset::parse_status(parameters)?,
            ))),
            CONFIG_RELAY_GET => Ok(Self::Config(Config::Relay(
                Relay::parse_get(parameters)?,
            ) )),
            _ => unimplemented!(),
        }
    }
}

#[derive(Format)]
pub enum Config {
    AppKey(AppKey),
    Beacon(Beacon),
    CompositionData(CompositionData),
    DefaultTTL(DefaultTTL),
    Friend(Friend),
    GATTProxy(GATTProxy),
    HeartbeatPublication(HeartbeatPublication),
    HeartbeatSubscription(HeartbeatSubscription),
    KeyRefreshPhase(KeyRefreshPhase),
    LowPowerNodePollTimeout(LowPowerNodePollTimeout),
    Model(Model),
    NetKey(NetKey),
    NetworkTransmit(NetworkTransmit),
    NodeIdentity(NodeIdentity),
    NodeReset(NodeReset),
    Relay(Relay),
    SIGModel(SIGModel),
    VendorModel(VendorModel),
}

impl Config {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::AppKey(inner) => inner.opcode(),
            Self::Beacon(inner) => inner.opcode(),
            Self::CompositionData(inner) => inner.opcode(),
            Self::DefaultTTL(inner) => inner.opcode(),
            Self::Friend(inner) => inner.opcode(),
            Self::GATTProxy(inner) => inner.opcode(),
            Self::HeartbeatPublication(inner) => inner.opcode(),
            Self::HeartbeatSubscription(inner) => inner.opcode(),
            Self::KeyRefreshPhase(inner) => inner.opcode(),
            Self::LowPowerNodePollTimeout(inner) => inner.opcode(),
            Self::Model(inner) => inner.opcode(),
            Self::NetKey(inner) => inner.opcode(),
            Self::NetworkTransmit(inner) => inner.opcode(),
            Self::NodeIdentity(inner) => inner.opcode(),
            Self::NodeReset(inner) => inner.opcode(),
            Self::Relay(inner) => inner.opcode(),
            Self::SIGModel(inner) => inner.opcode(),
            Self::VendorModel(inner) => inner.opcode(),
        }
    }
}

#[derive(Format)]
pub enum AppKey {
    Add,
    Delete,
    Get,
    List,
    Status,
    Update,
}

impl AppKey {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Add => CONFIG_APPKEY_ADD,
            Self::Delete => CONFIG_APPKEY_DELETE,
            Self::Get => CONFIG_APPKEY_GET,
            Self::List => CONFIG_APPKEY_LIST,
            Self::Status => CONFIG_APPKEY_STATUS,
            Self::Update => CONFIG_APPKEY_STATUS,
        }
    }
}

#[derive(Format)]
pub enum Beacon {
    Get,
    Set,
    Status,
}

impl Beacon {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_BEACON_GET,
            Self::Set => CONFIG_BEACON_SET,
            Self::Status => CONFIG_BEACON_STATUS,
        }
    }
}

#[derive(Format)]
pub enum CompositionData {
    Get,
    Status,
}

impl CompositionData {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_COMPOSITION_DATA_GET,
            Self::Status => CONFIG_COMPOSITION_DATA_STATUS,
        }
    }
}

#[derive(Format)]
pub enum DefaultTTL {
    Get,
    Set,
    Status,
}

impl DefaultTTL {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_DEFAULT_TTL_GET,
            Self::Set => CONFIG_DEFAULT_TTL_SET,
            Self::Status => CONFIG_DEFAULT_TTL_STATUS,
        }
    }
}

#[derive(Format)]
pub enum Friend {
    Get,
    Set,
    Status,
}

impl Friend {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_FRIEND_GET,
            Self::Set => CONFIG_FRIEND_SET,
            Self::Status => CONFIG_FRIEND_STATUS,
        }
    }
}

#[derive(Format)]
pub enum GATTProxy {
    Get,
    Set,
    Status,
}

impl GATTProxy {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_GATT_PROXY_GET,
            Self::Set => CONFIG_GATT_PROXY_SET,
            Self::Status => CONFIG_GATT_PROXY_STATUS,
        }
    }
}

#[derive(Format)]
pub enum HeartbeatPublication {
    Get,
    Set,
    Status,
}

impl HeartbeatPublication {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_HEARTBEAT_PUBLICATION_GET,
            Self::Set => CONFIG_HEARTBEAT_PUBLICATION_SET,
            Self::Status => CONFIG_HEARTBEAT_PUBLICATION_STATUS,
        }
    }
}

#[derive(Format)]
pub enum HeartbeatSubscription {
    Get,
    Set,
    Status,
}

impl HeartbeatSubscription {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_HEARTBEAT_SUBSCRIPTION_GET,
            Self::Set => CONFIG_HEARTBEAT_SUBSCRIPTION_SET,
            Self::Status => CONFIG_HEARTBEAT_SUBSCRIPTION_STATUS,
        }
    }
}

#[derive(Format)]
pub enum KeyRefreshPhase {
    Get,
    Set,
    Status,
}

impl KeyRefreshPhase {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_KEY_REFRESH_PHASE_GET,
            Self::Set => CONFIG_KEY_REFRESH_PHASE_SET,
            Self::Status => CONFIG_KEY_REFRESH_PHASE_STATUS,
        }
    }
}

#[derive(Format)]
pub enum LowPowerNodePollTimeout {
    Get,
    Status,
}

impl LowPowerNodePollTimeout {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_LOW_POWER_NODE_POLLTIMEOUT_GET,
            Self::Status => CONFIG_LOW_POWER_NODE_POLLTIMEOUT_STATUS,
        }
    }
}

#[derive(Format)]
pub enum Model {
    App(ModelApp),
    Publication(ModelPublication),
    Subscription(ModelSubscription),
}

impl Model {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::App(inner) => inner.opcode(),
            Self::Publication(inner) => inner.opcode(),
            Self::Subscription(inner) => inner.opcode(),
        }
    }
}

#[derive(Format)]
pub enum ModelApp {
    Bind,
    Status,
    Unbind,
}

impl ModelApp {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Bind => CONFIG_MODEL_APP_BIND,
            Self::Status => CONFIG_MODEL_APP_STATUS,
            Self::Unbind => CONFIG_MODEL_APP_UNBIND,
        }
    }
}

#[derive(Format)]
pub enum ModelPublication {
    Get,
    Status,
    VirtualAddressSet,
}

impl ModelPublication {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_MODEL_PUBLICATION_GET,
            Self::Status => CONFIG_MODEL_PUBLICATION_STATUS,
            Self::VirtualAddressSet => CONFIG_MODEL_PUBLICATION_VIRTUAL_ADDRESS_SET,
        }
    }
}

#[derive(Format)]
pub enum ModelSubscription {
    Add,
    Delete,
    DeleteAll,
    Overwrite,
    Status,
    VirtualAddressAdd,
    VirtualAddressDelete,
    VirtualAddressOverwrite,
}

impl ModelSubscription {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Add => CONFIG_MODEL_SUBSCRIPTION_ADD,
            Self::Delete => CONFIG_MODEL_SUBSCRIPTION_DELETE,
            Self::DeleteAll => CONFIG_MODEL_SUBSCRIPTION_DELETE_ALL,
            Self::Overwrite => CONFIG_MODEL_SUBSCRIPTION_OVERWRITE,
            Self::Status => CONFIG_MODEL_SUBSCRIPTION_STATUS,
            Self::VirtualAddressAdd => CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_ADD,
            Self::VirtualAddressDelete => CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_DELETE,
            Self::VirtualAddressOverwrite => CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_OVERWRITE,
        }
    }
}

#[derive(Format)]
pub enum NetKey {
    Add,
    Delete,
    Get,
    List,
    Status,
    Update,
}

impl NetKey {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Add => CONFIG_NETKEY_ADD,
            Self::Delete => CONFIG_NETKEY_DELETE,
            Self::Get => CONFIG_NETKEY_GET,
            Self::List => CONFIG_NETKEY_LIST,
            Self::Status => CONFIG_NETKEY_STATUS,
            Self::Update => CONFIG_NETKEY_UPDATE,
        }
    }
}

#[derive(Format)]
pub enum NetworkTransmit {
    Get,
    Set,
    Status,
}

impl NetworkTransmit {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_NETWORK_TRANSMIT_GET,
            Self::Set => CONFIG_NETWORK_TRANSMIT_SET,
            Self::Status => CONFIG_NETWORK_TRANSMIT_STATUS,
        }
    }
}

#[derive(Format)]
pub enum NodeIdentity {
    Get,
    Set,
    Status,
}

impl NodeIdentity {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_NODE_IDENTITY_GET,
            Self::Set => CONFIG_NODE_IDENTITY_SET,
            Self::Status => CONFIG_NODE_IDENTITY_STATUS,
        }
    }
}

#[derive(Format)]
pub enum NodeReset {
    Reset,
    Status,
}

impl NodeReset {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Reset => CONFIG_NODE_RESET,
            Self::Status => CONFIG_NODE_RESET_STATUS,
        }
    }

    fn parse_reset(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Reset)
        } else {
            Err(ParseError::InvalidLength)
        }
    }

    fn parse_status(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Status)
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

#[derive(Format)]
pub enum Relay {
    Get,
    Set,
    Status,
}

impl Relay {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_RELAY_GET,
            Self::Set => CONFIG_RELAY_SET,
            Self::Status => CONFIG_RELAY_STATUS,
        }
    }

    pub fn parse_get(parameters: &[u8]) -> Result<Self, ParseError> {
        if parameters.is_empty() {
            Ok(Self::Get)
        } else {
            Err(ParseError::InvalidLength)
        }
    }
}

#[derive(Format)]
pub enum SIGModel {
    App(SIGModelApp),
    Subscription(SIGModelSubscription),
}

impl SIGModel {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::App(inner) => inner.opcode(),
            Self::Subscription(inner) => inner.opcode(),
        }
    }
}

#[derive(Format)]
pub enum SIGModelApp {
    Get,
    List,
}

impl SIGModelApp {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_SIG_MODEL_APP_GET,
            Self::List => CONFIG_SIG_MODEL_APP_LIST,
        }
    }
}

#[derive(Format)]
pub enum SIGModelSubscription {
    Get,
    List,
}

impl SIGModelSubscription {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_SIG_MODEL_SUBSCRIPTION_GET,
            Self::List => CONFIG_SIG_MODEL_SUBSCRIPTION_LIST,
        }
    }
}

#[derive(Format)]
pub enum VendorModel {
    App(VendorModelApp),
    Susbcription(VendorModelSubscription),
}

impl VendorModel {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::App(inner) => inner.opcode(),
            Self::Susbcription(inner) => inner.opcode(),
        }
    }
}

#[derive(Format)]
pub enum VendorModelApp {
    Get,
    List,
}

impl VendorModelApp {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_VENDOR_MODEL_APP_GET,
            Self::List => CONFIG_VENDOR_MODEL_APP_LIST,
        }
    }
}

#[derive(Format)]
pub enum VendorModelSubscription {
    Get,
    List,
}

impl VendorModelSubscription {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => CONFIG_VENDOR_MODEL_SUBSCRIPTION_GET,
            Self::List => CONFIG_VENDOR_MODEL_SUBSCRIPTION_LIST,
        }
    }
}

#[derive(Format)]
pub enum Health {
    Attention(Attention),
    CurrentStatus,
    Fault(Fault),
    Period(Period),
}

impl Health {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::CurrentStatus => HEALTH_CURRENT_STATUS,
            Self::Attention(inner) => inner.opcode(),
            Self::Fault(inner) => inner.opcode(),
            Self::Period(inner) => inner.opcode(),
        }
    }
}

#[derive(Format)]
pub enum Attention {
    Get,
    Set,
    SetUnacknowledged,
}

impl Attention {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => HEALTH_ATTENTION_GET,
            Self::Set => HEALTH_ATTENTION_SET,
            Self::SetUnacknowledged => HEALTH_ATTENTION_SET_UNACKNOWLEDGED,
        }
    }
}

#[derive(Format)]
pub enum Fault {
    Clear,
    ClearUnacknowledged,
    Get,
    Status,
    Test,
    TestUnacknowledged,
}

impl Fault {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Clear => HEALTH_FAULT_CLEAR,
            Self::ClearUnacknowledged => HEALTH_FAULT_CLEAR_UNACKNOWLEDGED,
            Self::Get => HEALTH_FAULT_GET,
            Self::Status => HEALTH_FAULT_STATUS,
            Self::Test => HEALTH_FAULT_TEST,
            Self::TestUnacknowledged => HEALTH_FAULT_TEST_UNACKNOWLEDGED,
        }
    }
}

#[derive(Format)]
pub enum Period {
    Get,
    Set,
    SetUnacknowledged,
    Status,
}

impl Period {
    pub fn opcode(&self) -> Opcode {
        match self {
            Self::Get => HEALTH_PERIOD_GET,
            Self::Set => HEALTH_PERIOD_SET,
            Self::SetUnacknowledged => HEALTH_PERIOD_SET_UNACKNOWLEDGED,
            Self::Status => HEALTH_PERIOD_STATUS,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Opcode {
    OneOctet(u8),
    TwoOctet(u8, u8),
    ThreeOctet(u8, u8, u8),
}

impl Format for Opcode {
    fn format(&self, fmt: Formatter) {
        match self {
            Opcode::OneOctet(a) => {
                defmt::write!(fmt, "{:x}", a)
            }
            Opcode::TwoOctet(a, b) => {
                defmt::write!(fmt, "{:x} {:x}", a, b)
            }
            Opcode::ThreeOctet(a, b, c) => {
                defmt::write!(fmt, "{:x} {:x} {:x}", a, b, c)
            }
        }
    }
}

impl Opcode {
    pub fn matches(&self, data: &[u8]) -> bool {
        match self {
            Opcode::OneOctet(a) if data.len() >= 1 && data[0] == *a => true,
            Opcode::TwoOctet(a, b) if data.len() >= 2 && data[0] == *a && data[1] == *b => true,
            Opcode::ThreeOctet(a, b, c)
                if data.len() >= 3 && data[0] == *a && data[1] == *b && data[2] == *b =>
            {
                true
            }
            _ => false,
        }
    }

    pub fn opcode_len(&self) -> usize {
        match self {
            Opcode::OneOctet(_) => 1,
            Opcode::TwoOctet(_, _) => 2,
            Opcode::ThreeOctet(_, _, _) => 3,
        }
    }

    pub fn split(data: &[u8]) -> Option<(Opcode, &[u8])> {
        defmt::info!("opcode split {:x}", data);
        if data.is_empty() {
            None
        } else {
            if data[0] & 0b10000000 == 0 {
                // one octet
                Some((Opcode::OneOctet(data[0] & 0b00111111), &data[1..]))
            } else if data.len() >= 2 && data[0] & 0b11000000 == 0b01000000 {
                // two octet
                Some((Opcode::TwoOctet(data[0] & 0b00111111, data[1]), &data[2..]))
            } else if data.len() >= 3 && data[0] & 0b11000000 == 0b11000000 {
                // three octet
                Some((
                    Opcode::ThreeOctet(data[0] & 0b00111111, data[1], data[2]),
                    &data[3..],
                ))
            } else {
                None
            }
        }
    }
}

macro_rules! opcode {
    ($name:ident $o1:expr) => {
        pub const $name: Opcode = Opcode::OneOctet($o1);
    };

    ($name:ident $o1:expr, $o2:expr) => {
        pub const $name: Opcode = Opcode::TwoOctet($o1, $o2);
    };

    ($name:ident $o1:expr, $o2:expr, $o3:expr) => {
        pub const $name: Opcode = Opcode::ThreeOctet($o1, $o2, $o3);
    };
}

opcode!( CONFIG_APPKEY_ADD 0x00 );
opcode!( CONFIG_APPKEY_DELETE 0x80, 0x00 );
opcode!( CONFIG_APPKEY_GET 0x80, 0x01 );
opcode!( CONFIG_APPKEY_LIST 0x80, 0x02 );
opcode!( CONFIG_APPKEY_STATUS 0x80, 0x03 );
opcode!( CONFIG_APPKEY_UPDATE 0x01 );
opcode!( CONFIG_BEACON_GET 0x80, 0x09 );
opcode!( CONFIG_BEACON_SET 0x80, 0x0A );
opcode!( CONFIG_BEACON_STATUS 0x80, 0x0B );
opcode!( CONFIG_COMPOSITION_DATA_GET 0x80, 0x08 );
opcode!( CONFIG_COMPOSITION_DATA_STATUS 0x02 );
opcode!( CONFIG_CONFIG_MODEL_PUBLICATION_SET 0x03 );
opcode!( CONFIG_DEFAULT_TTL_GET 0x80, 0x0C );
opcode!( CONFIG_DEFAULT_TTL_SET 0x80, 0x0D );
opcode!( CONFIG_DEFAULT_TTL_STATUS 0x80, 0x0E );
opcode!( CONFIG_FRIEND_GET 0x80, 0x0F );
opcode!( CONFIG_FRIEND_SET 0x80, 0x10 );
opcode!( CONFIG_FRIEND_STATUS 0x80, 0x11 );
opcode!( CONFIG_GATT_PROXY_GET 0x80, 0x12 );
opcode!( CONFIG_GATT_PROXY_SET 0x80, 0x13 );
opcode!( CONFIG_GATT_PROXY_STATUS 0x80, 0x14 );
opcode!( CONFIG_HEARTBEAT_PUBLICATION_GET 0x80, 0x38 );
opcode!( CONFIG_HEARTBEAT_PUBLICATION_SET 0x80, 0x39 );
opcode!( CONFIG_HEARTBEAT_PUBLICATION_STATUS 0x06 );
opcode!( CONFIG_HEARTBEAT_SUBSCRIPTION_GET 0x80, 0x3A );
opcode!( CONFIG_HEARTBEAT_SUBSCRIPTION_SET 0x80, 0x3B );
opcode!( CONFIG_HEARTBEAT_SUBSCRIPTION_STATUS 0x80, 0x3C );
opcode!( CONFIG_KEY_REFRESH_PHASE_GET 0x80, 0x15 );
opcode!( CONFIG_KEY_REFRESH_PHASE_SET 0x80, 0x16 );
opcode!( CONFIG_KEY_REFRESH_PHASE_STATUS 0x80, 0x17 );
opcode!( CONFIG_LOW_POWER_NODE_POLLTIMEOUT_GET 0x80, 0x2D );
opcode!( CONFIG_LOW_POWER_NODE_POLLTIMEOUT_STATUS 0x80, 0x2E );
opcode!( CONFIG_MODEL_APP_BIND 0x80, 0x3D);
opcode!( CONFIG_MODEL_APP_STATUS 0x80, 0x3E);
opcode!( CONFIG_MODEL_APP_UNBIND 0x80, 0x3F);
opcode!( CONFIG_MODEL_PUBLICATION_GET 0x80, 0x18);
opcode!( CONFIG_MODEL_PUBLICATION_STATUS 0x80, 0x19);
opcode!( CONFIG_MODEL_PUBLICATION_VIRTUAL_ADDRESS_SET 0x80, 0x1A);
opcode!( CONFIG_MODEL_SUBSCRIPTION_ADD 0x80, 0x1B);
opcode!( CONFIG_MODEL_SUBSCRIPTION_DELETE 0x80, 0x1C);
opcode!( CONFIG_MODEL_SUBSCRIPTION_DELETE_ALL 0x80, 0x1D);
opcode!( CONFIG_MODEL_SUBSCRIPTION_OVERWRITE 0x80, 0x1E);
opcode!( CONFIG_MODEL_SUBSCRIPTION_STATUS 0x80, 0x1F);
opcode!( CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_ADD 0x80, 0x20);
opcode!( CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_DELETE 0x80, 0x21);
opcode!( CONFIG_MODEL_SUBSCRIPTION_VIRTUAL_ADDRESS_OVERWRITE 0x80, 0x22);
opcode!( CONFIG_NETKEY_ADD 0x80, 0x40);
opcode!( CONFIG_NETKEY_DELETE 0x80, 0x41);
opcode!( CONFIG_NETKEY_GET 0x80, 0x42);
opcode!( CONFIG_NETKEY_LIST 0x80, 0x43);
opcode!( CONFIG_NETKEY_STATUS 0x80, 0x44);
opcode!( CONFIG_NETKEY_UPDATE 0x80, 0x45);
opcode!( CONFIG_NETWORK_TRANSMIT_GET 0x80, 0x23);
opcode!( CONFIG_NETWORK_TRANSMIT_SET 0x80, 0x24);
opcode!( CONFIG_NETWORK_TRANSMIT_STATUS 0x80, 0x25);
opcode!( CONFIG_NODE_IDENTITY_GET 0x80, 0x46);
opcode!( CONFIG_NODE_IDENTITY_SET 0x80, 0x47);
opcode!( CONFIG_NODE_IDENTITY_STATUS 0x80, 0x48);
opcode!( CONFIG_NODE_RESET 0x80, 0x49);
opcode!( CONFIG_NODE_RESET_STATUS 0x80, 0x4A);
opcode!( CONFIG_RELAY_GET 0x80, 0x26);
opcode!( CONFIG_RELAY_SET 0x80, 0x27);
opcode!( CONFIG_RELAY_STATUS 0x80, 0x28);
opcode!( CONFIG_SIG_MODEL_APP_GET 0x80, 0x4B);
opcode!( CONFIG_SIG_MODEL_APP_LIST 0x80, 0x4C);
opcode!( CONFIG_SIG_MODEL_SUBSCRIPTION_GET 0x80, 0x29);
opcode!( CONFIG_SIG_MODEL_SUBSCRIPTION_LIST 0x80, 0x2A );
opcode!( CONFIG_VENDOR_MODEL_APP_GET 0x80, 0x4D );
opcode!( CONFIG_VENDOR_MODEL_APP_LIST 0x80, 0x4E );
opcode!( CONFIG_VENDOR_MODEL_SUBSCRIPTION_GET 0x80, 0x2B );
opcode!( CONFIG_VENDOR_MODEL_SUBSCRIPTION_LIST 0x80, 0x2C );

opcode!( HEALTH_ATTENTION_GET 0x80, 0x04 );
opcode!( HEALTH_ATTENTION_SET 0x80, 0x05 );
opcode!( HEALTH_ATTENTION_SET_UNACKNOWLEDGED 0x80, 0x06 );
opcode!( HEALTH_ATTENTION_STATUS 0x80, 0x07 );
opcode!( HEALTH_CURRENT_STATUS 0x04 );
opcode!( HEALTH_FAULT_CLEAR 0x80, 0x2F );
opcode!( HEALTH_FAULT_CLEAR_UNACKNOWLEDGED 0x80, 0x30 );
opcode!( HEALTH_FAULT_GET 0x80, 0x31 );
opcode!( HEALTH_FAULT_STATUS 0x05 );
opcode!( HEALTH_FAULT_TEST 0x80, 0x32 );
opcode!( HEALTH_FAULT_TEST_UNACKNOWLEDGED 0x80, 0x33 );
opcode!( HEALTH_PERIOD_GET 0x80, 0x34 );
opcode!( HEALTH_PERIOD_SET 0x80, 0x35 );
opcode!( HEALTH_PERIOD_SET_UNACKNOWLEDGED 0x80, 0x36 );
opcode!( HEALTH_PERIOD_STATUS 0x80, 0x37 );
