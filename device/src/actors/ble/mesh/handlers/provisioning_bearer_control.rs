use core::marker::PhantomData;

use rand_core::RngCore;

use crate::actors::ble::mesh::device::{Device, DeviceError};
use crate::drivers::ble::mesh::generic_provisioning::ProvisioningBearerControl;
use crate::drivers::ble::mesh::transport::Transport;

enum State {
    None,
    LinkOpen,
}

pub struct ProvisioningBearerControlHander<T, R>
where
    T: Transport + 'static,
    R: RngCore,
{
    state: State,
    pub(crate) link_id: Option<u32>,
    transaction_number: u8,
    _marker: PhantomData<(T, R)>,
}

impl<T, R> ProvisioningBearerControlHander<T, R>
where
    T: Transport + 'static,
    R: RngCore,
{
    pub(crate) fn new() -> Self {
        Self {
            state: State::None,
            link_id: None,
            transaction_number: 0x80,
            _marker: PhantomData,
        }
    }

    pub(crate) async fn handle(
        &mut self,
        device: &Device<T, R>,
        link_id: u32,
        pbc: &ProvisioningBearerControl,
    ) -> Result<(), DeviceError> {
        match pbc {
            ProvisioningBearerControl::LinkOpen(uuid) => {
                if *uuid != device.uuid {
                    // discard
                    return Ok(());
                }

                if matches!(self.link_id, None) {
                    defmt::trace!(">> LinkOpen({})", link_id);
                    self.link_id.replace(link_id);
                    device.tx_link_ack(link_id).await
                } else if matches!(self.link_id, Some(self_link_id) if link_id != self_link_id) {
                    // Something bad has happened?
                    defmt::debug!("LinkOpen with different link_id");
                    device.link_close();
                    self.link_id.take();
                    self.state = State::None;
                    Err(DeviceError::InvalidLink)
                } else {
                    // Re-ACK it.
                    device.tx_link_ack(link_id).await
                }
            }
            ProvisioningBearerControl::LinkAck => {
                // ignorable for this role
                Ok(())
            }
            ProvisioningBearerControl::LinkClose(reason) => {
                if let Some(_) = self.link_id {
                    defmt::trace!(">> LinkClose {}", reason);
                    device.link_close();
                    self.link_id.take();
                    self.state = State::None;
                }
                Ok(())
            }
        }
    }
}
