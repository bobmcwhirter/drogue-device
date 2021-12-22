use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::{Actor, ActorContext, ActorSpawner, Address, Inbox, Package};
use core::future::Future;
use heapless::Vec;
use nrf_softdevice::Softdevice;

use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::provisioning::Capabilities;
use crate::drivers::ble::mesh::PB_ADV;
use core::cell::UnsafeCell;
use core::ptr::slice_from_raw_parts;

pub struct BleMeshBearer<T>
where
    T: Transport + 'static,
{
    transport: T,
    start: ActorContext<Start<T>>,
    rx: ActorContext<Rx<T>>,
    tx: ActorContext<Tx<T>>,
    device: ActorContext<Device<T>>,
}

impl<T> BleMeshBearer<T>
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

impl<T> Package for BleMeshBearer<T>
where
    T: Transport + 'static,
{
    type Primary = Tx<T>;
    type Configuration = (Uuid, Capabilities);

    fn mount<S: ActorSpawner>(
        &'static self,
        config: Self::Configuration,
        spawner: S,
    ) -> Address<Self::Primary> {
        let _ = self.start.mount(spawner, Start(&self.transport));

        let tx = self.tx.mount(
            spawner,
            Tx {
                transport: &self.transport,
            },
        );

        let coordinator = self
            .device
            .mount(spawner, Device::new(config.0, config.1, tx));

        let rx = self.rx.mount(
            spawner,
            Rx {
                transport: &self.transport,
                handler: coordinator,
            },
        );

        tx
    }
}

struct Start<T: Transport + 'static>(&'static T);

impl<T: Transport + 'static> Actor for Start<T> {
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
            defmt::info!("transport START");
            self.0.start().await;
        }
    }
}

struct Rx<T>
where
    T: Transport + 'static,
{
    transport: &'static T,
    handler: Address<Device<T>>,
}

impl<T> Actor for Rx<T>
where
    T: Transport + 'static,
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
                                defmt::info!("<<<< Transmit {}", payload);
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
