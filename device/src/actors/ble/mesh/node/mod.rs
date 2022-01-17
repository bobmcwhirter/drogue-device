use crate::actors::ble::mesh::device::{Device, DeviceError};
use crate::actors::ble::mesh::pipeline::mesh::MeshContext;
use crate::actors::ble::mesh::pipeline::provisionable::ProvisionableContext;
use crate::actors::ble::mesh::pipeline::{Pipeline, PipelineContext};
use crate::actors::ble::mesh::vault::Vault;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::generic_provisioning::GenericProvisioningPDU;
use crate::drivers::ble::mesh::provisioning::{Capabilities, ProvisioningData, ProvisioningPDU};
use crate::{Actor, Address, Inbox};
use aes::Aes128;
use cmac::crypto_mac::{InvalidKeyLength, Output};
use cmac::{Cmac, Mac, NewMac};
use core::cell::RefCell;
use core::future::Future;
use embassy::time::{Duration, Ticker};
use futures::future::select;
use futures::{pin_mut, StreamExt};
use heapless::Vec;
use p256::PublicKey;
use rand_core::{CryptoRng, RngCore};

mod context;

pub trait Transmitter {
    type TransmitFuture<'m>: Future<Output = Result<(), DeviceError>>
    where
        Self: 'm;
    fn transmit_bytes<'m>(&self, bytes: &[u8]) -> Self::TransmitFuture<'m>;
}

pub trait Receiver {
    type ReceiveFuture<'m>: Future<Output = Result<&'m [u8], DeviceError>>
    where
        Self: 'm;
    fn receive_bytes<'m>(&self) -> Self::ReceiveFuture<'m>;
}

enum State {
    Unprovisioned,
    Provisioning,
    Provisioned,
}

pub struct Node<TX, RX, V, R>
where
    TX: Transmitter,
    RX: Receiver,
    V: Vault,
    R: RngCore + CryptoRng,
{
    state: State,
    link_id: RefCell<Option<u32>>,
    transaction_number: RefCell<Option<u8>>,
    //
    receiver: RX,
    transmitter: T,
    vault: RefCell<V>,
    rng: RefCell<R>,
    pipeline: RefCell<Pipeline>,
    ticker: Ticker,
}

impl<TX, RX, V, R> Node<TX, RX, V, R>
where
    TX: Transmitter + 'static,
    RX: Receiver + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    pub fn new(capabilities: Capabilities, receiver: RX, transmitter: T, vault: V, rng: R) -> Self {
        Self {
            state: State::Unprovisioned,
            link_id: RefCell::new(None),
            transaction_number: RefCell::new(None),
            receiver,
            transmitter,
            vault: RefCell::new(vault),
            rng: RefCell::new(rng),
            pipeline: RefCell::new(Pipeline::new(capabilities)),
            ticker: Ticker::every(Duration::from_secs(3)),
        }
    }

    async fn loop_unprovisioned<'m, M: Inbox<Self> + 'm>(
        &mut self,
        inbox: &'m mut M,
    ) -> Result<Option<State>, DeviceError> {
        let inbox_fut = inbox.next();
        let ticker_fut = self.ticker.next();

        pin_mut!(inbox_fut);
        pin_mut!(ticker_fut);

        //let result = match select(inbox_fut, ticker_fut).await {}
        Ok(None)
    }

    async fn loop_provisioning<'m, M: Inbox<Self> + 'm>(
        &mut self,
        inbox: &'m mut M,
    ) -> Result<Option<State>, DeviceError> {
        if let Some(mut message) = inbox.next().await {
            let mut pipeline = self.pipeline.borrow_mut();
            pipeline.process_inbound(self, message.message()).await?;
        }
        Ok(None)
    }

    async fn loop_provisioned<'m, M: Inbox<Self> + 'm>(
        &mut self,
        inbox: &'m mut M,
    ) -> Result<Option<State>, DeviceError> {
        Ok(None)
    }

    async fn run(&mut self) {
        loop {
            if let Ok(Some(next_state)) = match self.state {
                State::Unprovisioned => {
                    self.loop_unprovisioned().await;
                }
                State::Provisioning => {
                    self.loop_provisioning().await;
                }
                State::Provisioned => {
                    self.loop_provisioned().await;
                }
            } {
                self.state = next_state
            }
        }
    }
}

/*
impl<TX, RX, V, R> Actor for Node<TX, RX, V, R>
where
    TX: Transmitter + 'static,
    RX: Receiver + 'static,
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
            loop {
                if let Ok(Some(next_state)) = match self.state {
                    State::Unprovisioned => self.loop_unprovisioned(inbox).await,
                    State::Provisioning => self.loop_provisioning(inbox).await,
                    State::Provisioned => self.loop_provisioned(inbox).await,
                } {
                    self.state = next_state;
                }
            }
        }
    }
}
 */
