use core::future::Future;
use drogue_device::{Actor, Address, Inbox};
use embassy::time::{Delay, Duration, Ticker, Timer};
use futures::{pin_mut, StreamExt};
use futures::future::{Either, select};
use heapless::Vec;
use nrf_softdevice::ble::{peripheral, Connection};
use nrf_softdevice::{raw, Softdevice};

pub struct MeshTx {
    sd: &'static Softdevice,
    ticker: Ticker,
    unprovisioned: bool,
}

impl MeshTx {
    pub fn new(sd: &'static Softdevice) -> Self {
        Self {
            sd,
            ticker: Ticker::every(Duration::from_secs(2)),
            unprovisioned: true,
        }
    }
}

pub enum TransportMessage<'m> {
    UnprovisionedBeacon,
    Transmit(&'m [u8]),
}

impl Actor for MeshTx {
    type Message<'m> where Self: 'm = TransportMessage<'m>;
    type OnMountFuture<'m, M>
        where
            Self: 'm,
            M: 'm,
    = impl Future<Output=()> + 'm;
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, inbox: &'m mut M) -> Self::OnMountFuture<'m, M>
        where
            M: Inbox<Self> + 'm,
    {
        async move {
            loop {
                defmt::info!("loop");
                let inbox_fut = inbox.next();
                let ticker_fut = self.ticker.next();

                pin_mut!(inbox_fut);
                pin_mut!(ticker_fut);

                match select(inbox_fut, ticker_fut).await {
                    Either::Left((msg, _)) => {
                        defmt::info!("transmit");
                    },
                    Either::Right((_, _)) => {
                        defmt::info!("ticker expired");
                        if self.unprovisioned {
                            let mut adv_data: Vec<u8, 31> = Vec::new();

                            adv_data.extend_from_slice( &[
                                20,
                                ble_mesh::MESH_BEACON,
                                0x00,
                                0xBE, 0xEF, 0xCA, 0xFE, 0xBE, 0xEF, 0xDE, 0xCA, 0xFB, 0xAD, 0x48, 0x10, 0xd2, 0xe9, 0xBE, 0xEF,
                                0xa0, 0x40,
                            ]).unwrap();

                            let adv = peripheral::NonconnectableAdvertisement::NonscannableUndirected {
                                adv_data: &adv_data,
                            };

                            defmt::info!("Unprovisioned advert");
                            peripheral::advertise(self.sd, adv, &peripheral::Config {
                                max_events: Some(1),
                                .. Default::default()
                            }).await;
                            defmt::info!("Unprovisioned advert sent");
                        }
                    }
                }
            }
        }
    }
}