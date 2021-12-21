use crate::actors::ble::mesh::bearer::{BleMeshBearer, Tx};
use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::provisioning::Capabilities;
use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::{Actor, ActorContext, ActorSpawner, Address, Inbox, Package};
use core::future::Future;
use heapless::Vec;

pub mod bearer;
pub mod device;

pub struct BleMesh<T: Transport + 'static> {
    bearer: BleMeshBearer<T>,
    _noop: ActorContext<NoOp>,
}

impl<T: Transport + 'static> BleMesh<T> {
    pub fn new(transport: T) -> Self {
        Self {
            bearer: BleMeshBearer::new(transport),
            _noop: ActorContext::new(),
        }
    }
}

impl<T: Transport + 'static> Package for BleMesh<T> {
    type Primary = NoOp;
    type Configuration = (Uuid, Capabilities);

    fn mount<S: ActorSpawner>(
        &'static self,
        config: Self::Configuration,
        spawner: S,
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
