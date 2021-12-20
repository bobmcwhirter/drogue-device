use crate::actors::ble::mesh::bearer::{Tx, TxMessage};
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::{Actor, Address, Inbox};
use core::future::Future;
use embassy::time::{Duration, Ticker};
use futures::future::{select, Either};
use futures::{pin_mut, StreamExt};
use heapless::Vec;

pub struct Coordinator<T: Transport + 'static> {
    uuid: Uuid,
    tx: Address<Tx<T>>,
    ticker: Ticker,
}

impl<T: Transport + 'static> Coordinator<T> {
    pub fn new(uuid: Uuid, tx: Address<Tx<T>>) -> Self {
        Self {
            uuid,
            tx,
            ticker: Ticker::every(Duration::from_secs(3)),
        }
    }
}

impl<T: Transport + 'static> Handler for Address<Coordinator<T>> {
    fn handle<'m>(&self, message: Vec<u8, 384>) {
        self.notify(message);
    }
}

impl<T: Transport + 'static> Actor for Coordinator<T> {
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
                let inbox_fut = inbox.next();
                let ticker_fut = self.ticker.next();

                pin_mut!(inbox_fut);
                pin_mut!(ticker_fut);

                match select(inbox_fut, ticker_fut).await {
                    Either::Left((msg, _)) => {
                        defmt::info!("transmit");
                    }
                    Either::Right((_, _)) => {
                        defmt::info!("ticker expired");
                        self.tx
                            .request(TxMessage::UnprovisionedBeacon(self.uuid))
                            .unwrap()
                            .await;
                        defmt::info!("Unprovisioned advert sent");
                    }
                }
            }
        }
    }
}
