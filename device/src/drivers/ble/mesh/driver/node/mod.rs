use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::driver::pipeline::Pipeline;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::generic_provisioning::GenericProvisioningPDU;
use crate::drivers::ble::mesh::provisioning::{Capabilities, ProvisioningData, ProvisioningPDU};
use crate::drivers::ble::mesh::vault::Vault;
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
    fn transmit_bytes<'m>(&'m self, bytes: &'m [u8]) -> Self::TransmitFuture<'m>;
}

pub trait Receiver {
    type ReceiveFuture<'m>: Future<Output = Result<&'m [u8], DeviceError>>
    where
        Self: 'm;
    fn receive_bytes<'m>(&mut self) -> Self::ReceiveFuture<'m>;
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
    transmitter: TX,
    receiver: RX,
    vault: RefCell<V>,
    rng: RefCell<R>,
    pipeline: RefCell<Pipeline>,
    ticker: Ticker,
}

impl<TX, RX, V, R> Node<TX, RX, V, R>
where
    TX: Transmitter,
    RX: Receiver,
    V: Vault + 'static,
    R: RngCore + CryptoRng + 'static,
{
    pub fn new(
        capabilities: Capabilities,
        transmitter: TX,
        receiver: RX,
        vault: V,
        rng: R,
    ) -> Self {
        Self {
            state: State::Unprovisioned,
            link_id: RefCell::new(None),
            transaction_number: RefCell::new(None),
            transmitter,
            receiver,
            vault: RefCell::new(vault),
            rng: RefCell::new(rng),
            pipeline: RefCell::new(Pipeline::new(capabilities)),
            ticker: Ticker::every(Duration::from_secs(3)),
        }
    }

    async fn loop_unprovisioned(&mut self) -> Result<Option<State>, DeviceError> {
        let receive_fut = self.receiver.receive_bytes();
        let ticker_fut = self.ticker.next();

        pin_mut!(receive_fut);
        pin_mut!(ticker_fut);

        //let result = match select(inbox_fut, ticker_fut).await {}
        Ok(None)
    }

    async fn loop_provisioning(&mut self) -> Result<Option<State>, DeviceError> {
        let bytes = self.receiver.receive_bytes().await?;
        let mut pipeline = self.pipeline.borrow_mut();
        pipeline.process_inbound(self, bytes).await?;
        Ok(None)
    }

    async fn loop_provisioned(
        &mut self,
    ) -> Result<Option<State>, DeviceError> {
        Ok(None)
    }

    pub async fn run(&mut self) {
        loop {
            if let Ok(Some(next_state)) = match self.state {
                State::Unprovisioned => {
                    self.loop_unprovisioned().await
                }
                State::Provisioning => {
                    self.loop_provisioning().await
                }
                State::Provisioned => {
                    self.loop_provisioned().await
                }
            } {
                self.state = next_state;
            }
        }
    }
}
