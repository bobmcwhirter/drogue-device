use crate::actors::ble::mesh::bearer::{BleMeshBearer, Tx};
use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::{Actor, ActorSpawner, Address, Inbox, Package};
use core::future::Future;

pub mod bearer;

struct Coordinator<T: Transport + 'static> {
    tx: Address<Tx<T>>,
}

impl<T: Transport + 'static> Coordinator<T> {
    fn new(tx: Address<Tx<T>>) -> Self {
        Self { tx }
    }
}

impl<T: Transport + 'static> Handler for Address<Coordinator<T>> {
    fn handle(&self, message: &[u8]) {
        self.notify(message);
    }
}

impl<T: Transport + 'static> Actor for Coordinator<T> {
    type Message<'m> = &'m [u8];
    type OnMountFuture<'m, M> = impl Future<Output = ()> + 'm;

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
                match inbox.next().await {
                    Some(message) => {}
                    None => {}
                }
            }
        }
    }
}

pub struct BleMesh<T: Transport + 'static> {
    bearer: BleMeshBearer<T, Address<Coordinator<T>>>,
}

impl<T: Transport + 'static> BleMesh<T> {
    pub fn new(bearer: BleMeshBearer<T, Address<Coordinator<T>>>) -> Self {
        Self { bearer }
    }
}

impl<T: Transport + 'static> Package for BleMesh<T> {
    type Primary = ();

    fn mount<S: ActorSpawner>(
        &'static self,
        config: Self::Configuration,
        spawner: S,
    ) -> Address<Self::Primary> {
        todo!()
    }
}
