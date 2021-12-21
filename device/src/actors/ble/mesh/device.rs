use crate::actors::ble::mesh::bearer::TxMessage::Transmit;
use crate::actors::ble::mesh::bearer::{Tx, TxMessage};
use crate::drivers::ble::mesh::bearer::advertising;
use crate::drivers::ble::mesh::bearer::advertising::{PBAdvError, PDU};
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::generic_provisioning::{
    GenericProvisioningPDU, ProvisioningBearerControl, TransactionStart,
};
use crate::drivers::ble::mesh::provisioning::{Capabilities, ProvisioningPDU};
use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::drivers::ble::mesh::PB_ADV;
use crate::{Actor, Address, Inbox};
use core::future::Future;
use embassy::time::{Duration, Ticker};
use futures::future::{select, Either};
use futures::{pin_mut, StreamExt};
use heapless::Vec;

enum State {
    Unprovisioned,
    Provisioning(u32, ProvisioningState),
    Provisioned,
}

enum ProvisioningState {
    LinkOpen,
    TransactionStart,
    Invite,
}

pub struct Device<T: Transport + 'static> {
    uuid: Uuid,
    capabilities: Capabilities,
    state: State,
    tx: Address<Tx<T>>,
    ticker: Ticker,
}

impl<T: Transport + 'static> Device<T> {
    pub fn new(uuid: Uuid, capabilities: Capabilities, tx: Address<Tx<T>>) -> Self {
        Self {
            uuid,
            capabilities,
            state: State::Unprovisioned,
            tx,
            ticker: Ticker::every(Duration::from_secs(3)),
        }
    }

    async fn handle_provisioning_bearer_control(
        &mut self,
        link_id: u32,
        transaction_number: u8,
        pbc: ProvisioningBearerControl,
    ) {
        defmt::info!("provisioning bearer control {} {}", link_id, pbc);

        match pbc {
            ProvisioningBearerControl::LinkOpen(uuid) => {
                if uuid != self.uuid {
                    defmt::info!("drop wrong uuid");
                    return;
                }

                if matches!(self.state, State::Unprovisioned)
                    || matches!(
                        self.state,
                        State::Provisioning(link_id, ProvisioningState::LinkOpen)
                    )
                {
                    self.state = State::Provisioning(link_id, ProvisioningState::LinkOpen);
                    let ack = PDU {
                        link_id,
                        transaction_number,
                        pdu: GenericProvisioningPDU::ProvisioningBearerControl(
                            ProvisioningBearerControl::LinkAck,
                        ),
                    };

                    let mut xmit: Vec<u8, 128> = Vec::new();
                    ack.emit(&mut xmit);
                    self.tx.request(TxMessage::Transmit(&*xmit)).unwrap().await;
                }
            }
            ProvisioningBearerControl::LinkAck => {}
            ProvisioningBearerControl::LinkClose(reason) => {
                if !matches!(self.state, State::Provisioned) {
                    self.state = State::Unprovisioned
                }
            }
        }
    }

    async fn handle_transaction_start(
        &mut self,
        link_id: u32,
        transaction_number: u8,
        tx_start: TransactionStart,
    ) {
        if matches!(
            self.state,
            State::Provisioning(link_id, ProvisioningState::LinkOpen)
        ) {
            self.state = State::Provisioning(link_id, ProvisioningState::TransactionStart);

            let ack = PDU {
                link_id,
                transaction_number,
                pdu: GenericProvisioningPDU::TransactionAck,
            };

            let mut xmit: Vec<u8, 128> = Vec::new();
            ack.emit(&mut xmit);
            self.tx.request(TxMessage::Transmit(&*xmit)).unwrap().await;

            let pdu = ProvisioningPDU::parse(&*tx_start.data).unwrap();
            defmt::info!("tx {}", pdu);

            match pdu {
                ProvisioningPDU::Invite(invite) => {
                    defmt::info!("INVITED sending capabilities");
                    self.state = State::Provisioning(link_id, ProvisioningState::Invite);

                    let mut xmit: Vec<u8, 128> = Vec::new();
                    ProvisioningPDU::Capabilities(self.capabilities.clone()).emit(&mut xmit);
                    self.tx.request(TxMessage::Transmit(&*xmit)).unwrap().await;
                }
                _ => {
                    defmt::info!("unhandled PDU {}", pdu)
                }
            }
        }
    }
}

impl<T: Transport + 'static> Handler for Address<Device<T>> {
    fn handle<'m>(&self, message: Vec<u8, 384>) {
        self.notify(message);
    }
}

impl<T: Transport + 'static> Actor for Device<T> {
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
                    Either::Left((ref mut msg, _)) => match msg {
                        Some(message) => {
                            let data = message.message();
                            defmt::info!("received data {:x}", data);
                            if data.len() >= 2 && data[1] == PB_ADV {
                                let pdu = advertising::PDU::parse(data);
                                defmt::info!("received pdu {}", pdu);

                                if let Ok(pdu) = pdu {
                                    match pdu.pdu {
                                        GenericProvisioningPDU::TransactionStart(tx_start) => {
                                            self.handle_transaction_start(
                                                pdu.link_id,
                                                pdu.transaction_number,
                                                tx_start,
                                            )
                                            .await;
                                        }
                                        GenericProvisioningPDU::TransactionContinuation {
                                            ..
                                        } => {
                                            defmt::info!("transaction continuation");
                                        }
                                        GenericProvisioningPDU::TransactionAck => {
                                            defmt::info!("transaction ack");
                                        }
                                        GenericProvisioningPDU::ProvisioningBearerControl(
                                            bearer_control,
                                        ) => {
                                            self.handle_provisioning_bearer_control(
                                                pdu.link_id,
                                                pdu.transaction_number,
                                                bearer_control,
                                            )
                                            .await;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            defmt::info!("got a None?")
                        }
                    },
                    Either::Right((_, _)) => {
                        if matches!(self.state, State::Unprovisioned) {
                            self.tx
                                .request(TxMessage::UnprovisionedBeacon(self.uuid))
                                .unwrap()
                                .await;
                        }
                    }
                }
            }
        }
    }
}
