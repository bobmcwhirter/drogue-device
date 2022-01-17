mod mesh_node;

use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::{Actor, ActorContext, ActorSpawner, Address, Inbox, Package};
use core::future::Future;

use crate::actors::ble::mesh::mesh_node::MeshNode;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::provisioning::Capabilities;
use crate::drivers::ble::mesh::storage::Storage;
use crate::drivers::ble::mesh::vault::Vault;
use core::marker::PhantomData;
use core::cell::RefCell;
use rand_core::{CryptoRng, RngCore};
use heapless::Vec;

pub struct BleMesh<T, V, R>
where
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    capabilities: RefCell<Option<Capabilities>>,
    transport: T,
    vault: RefCell<Option<V>>,
    rng: RefCell<Option<R>>,
    start: ActorContext<Start<T>>,
    rx: ActorContext<Rx<T, V, R>>,
    node: ActorContext<MeshNode<T, V, R>>,
    //
    noop: ActorContext<NoOp>,
}

impl<T,V, R> BleMesh<T,V, R>
where
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    pub fn new(capabilities: Capabilities, transport: T, vault: V, rng: R) -> Self {
        Self {
            capabilities: RefCell::new(Some(capabilities)),
            transport,
            vault: RefCell::new(Some(vault)),
            rng: RefCell::new(Some(rng)),
            start: ActorContext::new(),
            rx: ActorContext::new(),
            node: ActorContext::new(),
            noop: ActorContext::new(),
        }
    }
}

impl<T, V, R> Package for BleMesh<T, V, R>
where
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    type Primary = NoOp;
    type Configuration = ();

    fn mount<AS: ActorSpawner>(
        &'static self,
        config: Self::Configuration,
        spawner: AS,
    ) -> Address<Self::Primary> {
        let _ = self.start.mount(spawner, Start(&self.transport));

        let node = self.node.mount(
            spawner,
            MeshNode::new(
                self.capabilities.borrow_mut().take().unwrap(),
                &self.transport,
                self.vault.borrow_mut().take().unwrap(),
                self.rng.borrow_mut().take().unwrap(),
            ),
        );

        let _rx = self.rx.mount(
            spawner,
            Rx {
                transport: &self.transport,
                handler: node,
            },
        );

        self.noop.mount(spawner, NoOp{})
    }
}

struct Start<T: Transport + 'static>(&'static T);

impl<T> Actor for Start<T>
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
            self.0.start().await;
        }
    }
}

struct Rx<T, V, R>
where
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    transport: &'static T,
    handler: Address<MeshNode<T, V, R>>,
}

impl<T, V, R> Actor for Rx<T, V, R>
where
    T: Transport + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
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

pub struct NoOp {

}

impl Actor for NoOp {
    type Message<'m> = ();
    type OnMountFuture<'m, M>
        where
            Self: 'm,
            M: 'm,
    = impl Future<Output = ()> + 'm;

    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M> where M: Inbox<Self> + 'm {
         async move {

         }
    }
}


impl<T, V, R> Handler for Address<MeshNode<T, V, R>>
    where
        T: Transport + 'static,
        V: Vault + 'static,
        R: RngCore + CryptoRng + 'static,
{
    fn handle<'m>(&self, message: Vec<u8, 384>) {
        self.notify(message).ok();
    }
}