use crate::actors::ble::mesh::bearer::{Tx, TxMessage};
use crate::drivers::ble::mesh::storage::Storage;
use crate::drivers::ble::mesh::transport::Transport;
use crate::{Actor, Address, Inbox};
use core::future::Future;
use core::marker::PhantomData;
use heapless::Vec;
use rand_core::{CryptoRng, RngCore};

use crate::actors::ble::mesh::device::DeviceError;
use crate::drivers::ble::mesh::bearer::advertising;
use crate::drivers::ble::mesh::{MESH_MESSAGE, PB_ADV};
use embassy::time::{Duration, Ticker};
use futures::future::{select, Either};
use futures::{pin_mut, StreamExt};

enum State {
    Unprovisioned,
    Provisioning,
    Provisioned,
}

pub struct Node<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    ticker: Ticker,
    state: State,
    storage: S,
    tx: Address<Tx<T>>,
    _marker: PhantomData<(T, R, S)>,
}

impl<T, R, S> Node<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    pub fn new(storage: S, tx: Address<Tx<T>>) -> Self {
        Self {
            ticker: Ticker::every(Duration::from_secs(3)),
            state: State::Unprovisioned,
            storage,
            tx,
            _marker: PhantomData,
        }
    }

    async fn initialize(&mut self) -> Result<(), DeviceError>{
        Ok(())
    }

    async fn do_unprovisioned<'m, M: Inbox<Self> + 'm>(&mut self, inbox: &mut M) -> Result<Option<State>, DeviceError> {
        let inbox_fut = inbox.next();
        let ticker_fut = self.ticker.next();

        pin_mut!(inbox_fut);
        pin_mut!(ticker_fut);

        match select(inbox_fut, ticker_fut).await {
            Either::Left((ref mut msg, _)) => match msg {
                Some(message) => {
                    let data = message.message();
                    if data.len() >= 2 {
                        if data[1] == PB_ADV {
                            let pdu = advertising::PDU::parse(data);
                            if let Ok(pdu) = pdu {
                                Ok(Some(State::Provisioning))
                                //self.handle_pdu(pdu).await
                            } else {
                                Err(DeviceError::InvalidPacket)
                            }
                        } else {
                            Err(DeviceError::InvalidPacket)
                        }
                    } else {
                        // Not long enough to bother with.
                        Err(DeviceError::InvalidPacket)
                    }
                }
                _ => {
                    // ignore
                    Ok(None)
                }
            },
            Either::Right((_, _)) => {
                Ok(None)
                /*
                if matches!(self.state, State::Unprovisioned) {
                    self.tx
                        .request(TxMessage::UnprovisionedBeacon(self.uuid))
                        .unwrap()
                        .await;
                    Ok(())
                } else {
                    Ok(())
                }
                 */
            }
        }
    }

    async fn do_provisioning(&mut self) {}

    async fn do_provisioned(&mut self) {}
}

impl<T, R, S> Actor for Node<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
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
            if let Err(e) = self.initialize().await {
                defmt::error!("Unable to initialize BLE-Mesh stack. Stuff isn't going to work.")
            }

            loop {
                match self.state {
                    State::Unprovisioned => {
                        self.do_unprovisioned(inbox).await;
                    }
                    State::Provisioning => {
                        self.do_provisioning().await;
                    }
                    State::Provisioned => {
                        self.do_provisioned().await;
                    }
                }
            }
        }
    }
}
