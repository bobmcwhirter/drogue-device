use crate::drivers::ble::mesh::transport::Transport;
use crate::{Actor, ActorContext, ActorSpawner, Address, Inbox, Package};
use core::future::Future;

use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::provisioning::Capabilities;
use core::marker::PhantomData;
use rand_core::{CryptoRng, RngCore};
use crate::drivers::ble::mesh::storage::Storage;

pub struct MeshTransport<T>
where
    T: Transport + 'static,
{
    transport: T,
}

impl<T> MeshTransport<T>
where
    T: Transport + 'static,
{
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            start: ActorContext::new(),
            rx: ActorContext::new(),
            tx: ActorContext::new(),
            device: ActorContext::new(),
        }
    }
}

impl<T, R, S> Package for BleMeshBearer<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    type Primary = Tx<T>;
    type Configuration = (R, S, Uuid, Capabilities);

    fn mount<AS: ActorSpawner>(
        &'static self,
        config: Self::Configuration,
        spawner: AS,
    ) -> Address<Self::Primary> {
        let _ = self.start.mount(spawner, Start(&self.transport));

        let tx = self.tx.mount(
            spawner,
            Tx {
                transport: &self.transport,
            },
        );

        let device = self
            .device
            .mount(spawner, Device::new(config.0, config.1, config.2, config.3, tx));

        let _rx = self.rx.mount(
            spawner,
            Rx {
                transport: &self.transport,
                handler: device,
            },
        );

        tx
    }
}

struct Start<T: Transport + 'static>(&'static T);

impl<T> Actor for Start<T> where T: Transport + 'static {
    type OnMountFuture<'m, M>
    where
        Self: 'm,
        M: 'm,
    = impl Future<Output = ()> + 'm;

    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {
            self.0.start().await;
        }
    }
}

struct Rx<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    transport: &'static T,
    handler: Address<Device<T,R, S>>,
}

impl<T, R, S> Actor for Rx<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    type OnMountFuture<'m, M>
    where
        Self: 'm,
        M: 'm,
    = impl Future<Output = ()> + 'm;
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
    {
        async move {
            self.transport.start_receive(&self.handler).await;
        }
    }
}

pub struct Tx<T: Transport + 'static> {
    transport: &'static T,
}

pub enum TxMessage<'m> {
    UnprovisionedBeacon(Uuid),
    Transmit(&'m [u8]),
}

impl<T: Transport + 'static> Actor for Tx<T> {
    type Message<'m>
    where
        Self: 'm,
    = TxMessage<'m>;
    type OnMountFuture<'m, M>
    where
        Self: 'm,
        M: 'm,
    = impl Future<Output = ()> + 'm;
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
                    Some(ref mut message) => {
                        let tx = message.message();
                        match tx {
                            TxMessage::UnprovisionedBeacon(uuid) => {
                                self.transport.send_unprovisioned_beacon(*uuid).await;
                            }
                            TxMessage::Transmit(payload) => {
                                self.transport.transmit(payload).await;
                            }
                        }
                    }
                    None => {}
                }
            }
        }
    }
}
