use core::marker::PhantomData;
use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::provisioning::ProvisioningPDU;
use crate::drivers::ble::mesh::transport::Transport;

enum State {
    None,
    Invite,
    Capabilities,
    Start,
    PublicKey,
    InputComplete,
    Confirmation,
    Random,
    Data,
    Complete,
    Failed,
}

pub struct ProvisioningHandler<T:Transport + 'static> {
    state: State,
    _marker: PhantomData<T>
}

impl<T: Transport + 'static> ProvisioningHandler<T> {
    pub(crate) fn new() -> Self {
        Self {
            state: State::None,
            _marker: PhantomData
        }
    }

    pub(crate) async fn handle(&mut self, device: &Device<T>, pdu: ProvisioningPDU) -> Result<(), ()> {
        match pdu {
            ProvisioningPDU::Invite(invite) => {
                defmt::info!("Received INVITE");
                self.state = State::Invite;
                device.tx_capabilities()?;
            }
            ProvisioningPDU::Capabilities(_) => {}
            ProvisioningPDU::Start { .. } => {}
            ProvisioningPDU::PublicKey { .. } => {}
            ProvisioningPDU::InputComplete => {}
            ProvisioningPDU::Confirmation { .. } => {}
            ProvisioningPDU::Random { .. } => {}
            ProvisioningPDU::Data { .. } => {}
            ProvisioningPDU::Complete => {}
            ProvisioningPDU::Failed { .. } => {}
        }
        Ok(())
    }
}