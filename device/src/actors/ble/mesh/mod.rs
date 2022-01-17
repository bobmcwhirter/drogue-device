use crate::drivers::ble::mesh::driver::node::{Node, Receiver, Transmitter};
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::provisioning::Capabilities;
use crate::drivers::ble::mesh::transport::Transport;
use crate::drivers::ble::mesh::vault::Vault;
use crate::{Actor, Address, Inbox};
use core::future::Future;
use core::marker::PhantomData;
use heapless::Vec;
use rand_core::{CryptoRng, RngCore};

pub struct BleMesh<T, V, R>
where
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    capabilities: Option<Capabilities>,
    tx: Option<TransportTransmitter<T>>,
    vault: Option<V>,
    rng: Option<R>,
}

impl<T, V, R> BleMesh<T, V, R>
where
    T: Transport,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    pub fn new(capabilities: Capabilities, transport: T, vault: V, rng: R) -> Self {
        let tx = TransportTransmitter {
            transport,
        };
        Self {
            capabilities: Some(capabilities),
            tx: Some(tx),
            vault: Some(vault),
            rng: Some(rng),
        }
    }
}

struct InboxReceiver<'i, I, T, V, R>
where
    I: Inbox<BleMesh<T, V, R>> + 'i,
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    inbox: &'i mut I,
    _marker: PhantomData<(T, V, R)>,
}

impl<'i, I, T, V, R> Receiver for InboxReceiver<'i, I, T, V, R>
where
    I: Inbox<BleMesh<T, V, R>> +'i,
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    type ReceiveFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<&'m [u8], DeviceError>>;

    fn receive_bytes<'m>(&mut self) -> Self::ReceiveFuture<'m> {
        async move { todo!() }
    }
}

struct TransportTransmitter<T>
where
    T: Transport + 'static,
{
    transport: T,
}

impl<T> Transmitter for TransportTransmitter<T>
where
    T: Transport,
{
    type TransmitFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn transmit_bytes<'m>(&'m self, bytes: &'m [u8]) -> Self::TransmitFuture<'m> {
        async move {
            self.transport.transmit(bytes).await;
            Ok(())
        }
    }
}

impl<T, V, R> Actor for BleMesh<T, V, R>
where
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    type Message<'m> = Vec<u8, 384>;
    type OnMountFuture<'m, M>
    where
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
            let mut rx = InboxReceiver {
                inbox,
                _marker: PhantomData,
            };

            let mut node = Node::new(
                self.capabilities.take().unwrap(),
                self.tx.take().unwrap(),
                rx,
                self.vault.take().unwrap(),
                self.rng.take().unwrap(),
            );
        }
    }
}
