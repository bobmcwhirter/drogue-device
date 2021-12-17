use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::{Actor, ActorContext, ActorSpawner, Address, Inbox, Package};
use core::future::Future;
use heapless::Vec;
use nrf_softdevice::Softdevice;

use crate::drivers::ble::mesh::PB_ADV;
use core::cell::UnsafeCell;
use core::ptr::slice_from_raw_parts;

pub struct BleMeshBearer<T, H>
where
    T: Transport + 'static,
    H: Handler + 'static,
{
    transport: T,
    start: ActorContext<Start<T>>,
    rx: ActorContext<Rx<T, H>>,
    tx: ActorContext<Tx<T>>,
    handler: &'static H,
}

impl<T, H> BleMeshBearer<T, H>
where
    T: Transport + 'static,
    H: Handler + 'static,
{
    pub fn new(transport: T, handler: &'static H) -> Self {
        Self {
            transport,
            start: ActorContext::new(),
            rx: ActorContext::new(),
            tx: ActorContext::new(),
            handler,
        }
    }
}

impl<T, H> Package for BleMeshBearer<T, H>
where
    T: Transport + 'static,
    H: Handler + 'static,
{
    type Primary = Tx<T>;
    type Configuration = ();

    fn mount<S: ActorSpawner>(
        &'static self,
        _config: Self::Configuration,
        spawner: S,
    ) -> Address<Self::Primary> {
        let _ = self.start.mount(spawner, Start(&self.transport));

        self.rx.mount(
            spawner,
            Rx {
                transport: &self.transport,
                handler: self.handler,
            },
        );

        self.tx.mount(
            spawner,
            Tx {
                transport: &self.transport,
            },
        )
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
            self.0.start().await;
        }
    }
}

struct Rx<T: Transport + 'static, H: Handler + 'static> {
    transport: &'static T,
    handler: &'static H,
}

impl<T: Transport + 'static, H: Handler + 'static> Actor for Rx<T, H> {
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
            self.transport.start_receive(self.handler).await;
        }
    }
}

pub struct Tx<T: Transport + 'static> {
    transport: &'static T,
}

pub struct Transmit<'m>(pub &'m [u8]);

impl<T: Transport + 'static> Actor for Tx<T> {
    type Message<'m>
    where
        Self: 'm,
    = Transmit<'m>;
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
                defmt::info!("loop");
                match inbox.next().await {
                    Some(ref mut message) => {
                        let xmit = message.message();
                        self.transport.transmit(xmit.0).await;
                    }
                    None => {}
                }
            }
        }
    }
}
