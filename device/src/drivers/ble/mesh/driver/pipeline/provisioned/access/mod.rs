use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::access::{AccessMessage, Config, Health};

pub trait AccessContext {

}

pub struct Access {

}

impl Default for Access {
    fn default() -> Self {
        Self {

        }
    }
}

impl Access {
    pub async fn process_inbound<C:AccessContext>(&mut self, ctx: &C, message: AccessMessage) -> Result<Option<AccessMessage>, DeviceError> {
        match message {
            AccessMessage::Config(config) => {
                match config {
                    Config::AppKey(_) => {}
                    Config::Beacon(_) => {}
                    Config::CompositionData(_) => {}
                    Config::DefaultTTL(_) => {}
                    Config::Friend(_) => {}
                    Config::GATTProxy(_) => {}
                    Config::HeartbeatPublication(_) => {}
                    Config::HeartbeatSubscription(_) => {}
                    Config::KeyRefreshPhase(_) => {}
                    Config::LowPowerNodePollTimeout(_) => {}
                    Config::Model(_) => {}
                    Config::NetKey(_) => {}
                    Config::NetworkTransmit(_) => {}
                    Config::NodeIdentity(_) => {}
                    Config::NodeReset(_) => {}
                    Config::Relay(_) => {}
                    Config::SIGModel(_) => {}
                    Config::VendorModel(_) => {}
                }
            }
            AccessMessage::Health(health) => {
                match health {
                    Health::Attention(_) => {}
                    Health::CurrentStatus => {}
                    Health::Fault(_) => {}
                    Health::Period(_) => {}
                }
            }
        }

        Ok(None)
    }

}