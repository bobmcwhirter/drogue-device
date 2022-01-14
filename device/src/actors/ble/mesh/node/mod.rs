use crate::actors::ble::mesh::device::{Device, DeviceError};
use crate::actors::ble::mesh::pipeline::mesh::MeshContext;
use crate::actors::ble::mesh::pipeline::provisionable::ProvisionableContext;
use crate::actors::ble::mesh::pipeline::provisioning_bearer::ProvisioningBearerContext;
use crate::actors::ble::mesh::pipeline::transaction::TransactionContext;
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

enum State {
    Unprovisioned,
    Provisioning,
    Provisioned,
}

pub struct Node<T, V, R>
where
    T: Transmitter,
    V: Vault,
    R: RngCore + CryptoRng,
{
    state: State,
    link_id: Option<u32>,
    transaction_number: Option<u8>,
    //
    transmitter: T,
    vault: V,
    rng: R,
    pipeline: Pipeline,
    ticker: Ticker,
}

impl<T, V, R> Node<T, V, R>
where
    T: Transmitter + 'static,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    pub fn new(capabilities: Capabilities, transmitter: T, vault: V, rng: R) -> Self {
        Self {
            state: State::Unprovisioned,
            link_id: None,
            transaction_number: None,
            transmitter,
            vault,
            rng,
            pipeline: Pipeline::new(capabilities),
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
            self.pipeline.process_inbound( self, message.message() ).await?;
        }
        Ok(None)
    }

    async fn loop_provisioned<'m, M: Inbox<Self> + 'm>(
        &mut self,
        inbox: &'m mut M,
    ) -> Result<Option<State>, DeviceError> {
        Ok(None)
    }
}

impl<T, V, R> Actor for Node<T, V, R>
where
    T: Transmitter + 'static,
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
