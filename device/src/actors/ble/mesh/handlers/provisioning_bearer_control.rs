use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::generic_provisioning::{
    GenericProvisioningPDU, ProvisioningBearerControl,
};
use crate::drivers::ble::mesh::transport::Transport;
use core::marker::PhantomData;
use heapless::Vec;

enum State {
    None,
    LinkOpen,
}

pub struct ProvisioningBearerControlHander<T: Transport + 'static> {
    state: State,
    pub(crate) link_id: Option<u32>,
    transaction_number: u8,
    _marker: PhantomData<T>,
}

impl<T: Transport + 'static> ProvisioningBearerControlHander<T> {
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
        device: &Device<T>,
        link_id: u32,
        pbc: &ProvisioningBearerControl,
    ) {
        match pbc {
            ProvisioningBearerControl::LinkOpen(uuid) => {
                if *uuid != device.uuid {
                    // discard
                }

                if matches!(self.link_id, None) || matches!(self.link_id, Some(link_id)) {
                    defmt::info!(">> LinkOpen");
                    self.link_id.replace(link_id);
                    device.tx_link_ack(link_id).await;
                }
            }
            ProvisioningBearerControl::LinkAck => {
                // ignorable for this role
            }
            ProvisioningBearerControl::LinkClose(reason) => {
                defmt::info!(">> LinkClose");
                self.link_id.take();
                self.state = State::None;
            }
        }
    }
}
