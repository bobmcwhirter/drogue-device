use crate::actors::ble::mesh::bearer::BleMeshBearer;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::provisioning::Capabilities;
use crate::drivers::ble::mesh::transport::Transport;
use crate::{Actor, ActorContext, ActorSpawner, Address, Inbox, Package};
use core::future::Future;
use rand_core::{CryptoRng, RngCore};
use crate::drivers::ble::mesh::storage::Storage;

pub mod bearer;
pub mod device;
mod handlers;
mod key_manager;
mod configuration_manager;

pub struct BleMesh<T, R, S>
where
    T: Transport + 'static,
    R: CryptoRng + RngCore + 'static,
    S: Storage + 'static,
{
    bearer: BleMeshBearer<T, R, S>,
    _noop: ActorContext<NoOp>,
}

impl<T, R, S> BleMesh<T, R, S>
where
    T: Transport + 'static,
    R: CryptoRng + RngCore + 'static,
    S: Storage + 'static,
{
    pub fn new(transport: T) -> Self {
        Self {
            bearer: BleMeshBearer::new(transport),
            _noop: ActorContext::new(),
        }
    }
}

impl<T, R, S> Package for BleMesh<T, R, S>
where
    T: Transport + 'static,
    R: CryptoRng + RngCore + 'static,
    S: Storage + 'static,
{
    type Primary = NoOp;
    type Configuration = (R, S, Uuid, Capabilities);

    fn mount<AS: ActorSpawner>(
        &'static self,
        config: Self::Configuration,
        spawner: AS,
    ) -> Address<Self::Primary> {
        let _ = self.bearer.mount(config, spawner);
        self._noop.mount(spawner, NoOp {})
    }
}

pub struct NoOp;

impl Actor for NoOp {
    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl Future<Output = ()> + 'm;

    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {}
    }
}
