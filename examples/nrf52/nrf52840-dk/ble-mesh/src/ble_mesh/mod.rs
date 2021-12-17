use core::future::Future;
use ble_mesh::transport::{Receiver, Transport};
use drogue_device::{Actor, ActorContext, ActorSpawner, Address, Inbox, Package};
use heapless::Vec;
use nrf_softdevice::Softdevice;
use crate::ble_mesh::tx::MeshTx;
use crate::ble_mesh::rx::MeshRx;

mod tx;
mod rx;
pub mod nrf52;

pub struct MeshBleService<T: Transport>
{
    mesh: ActorContext<Mesh>,
    transport: T,
}

impl<T: Transport> MeshBleService<T>
{
    pub fn new() -> Self {
        Self {
            transport: T::new(Receiver {}),
            mesh: ActorContext::new(),
        }
    }
}

impl<T: Transport> Package for MeshBleService<T>
{
    type Primary = Mesh;

    fn mount<S: ActorSpawner>(
        &'static self,
        _config: Self::Configuration,
        spawner: S,
    ) -> Address<Self::Primary> {
        self.transport.start(spawner.embassy_spawner());
        self.mesh.mount( spawner, Mesh { } )
    }
}

pub struct Mesh {

}

impl Actor for Mesh {
    type OnMountFuture<'m, M>
        where
            Self: 'm,
            M: 'm,
    = impl Future<Output=()> + 'm;

    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M> where M: Inbox<Self> + 'm {
        async move {

        }
    }
}
