use core::future::Future;
use crate::{Actor, Address, Inbox};
use crate::drivers::ble::mesh::provisioning::ProvisioningPDU;

pub struct Provisioning {

}

impl Actor for Provisioning {
    type Message<'m> = ProvisioningPDU;
    type OnMountFuture<'m, M>
        where
            M: 'm,
    = impl Future<Output=()> + 'm;

    fn on_mount<'m, M>(
        &'m mut self,
        _: Address<Self>,
        inbox: &'m mut M,
    ) -> Self::OnMountFuture<'m, M>
        where
            M: Inbox<Self> + 'm,
    {
        async move {
            loop {
                let mut message = inbox.next().await;
                if let Some(mut message) = message {

                }
            }
        }
    }
}