use super::provisioning_bearer_control::ProvisioningBearerControlHander;
use crate::actors::ble::mesh::bearer::TxMessage::Transmit;
use crate::actors::ble::mesh::bearer::{Tx, TxMessage};
use crate::actors::ble::mesh::transaction::TransactionHandler;
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
use core::cell::RefCell;
use core::future::Future;
use core::sync::atomic::{AtomicU8, Ordering};
use embassy::time::{Duration, Ticker};
use futures::future::{select, Either};
use futures::{pin_mut, StreamExt};
use heapless::Vec;

enum State {
    Unprovisioned,
    Provisioning,
    Provisioned,
}

enum ProvisioningState {
    LinkOpen,
    TransactionStart,
    Invite,
}

pub struct Device<T: Transport + 'static> {
    pub(crate) uuid: Uuid,
    capabilities: Capabilities,
    transaction_number: AtomicU8,
    state: State,
    tx: Address<Tx<T>>,
    ticker: Ticker,
    // Handlers
    provisioning_bearer_control: RefCell<ProvisioningBearerControlHander<T>>,
    transaction: RefCell<TransactionHandler<T>>,
}

impl<T: Transport + 'static> Device<T> {
    pub fn new(uuid: Uuid, capabilities: Capabilities, tx: Address<Tx<T>>) -> Self {
        Self {
            uuid,
            capabilities,
            transaction_number: AtomicU8::new(0x80),
            state: State::Unprovisioned,
            tx,
            ticker: Ticker::every(Duration::from_secs(3)),
            provisioning_bearer_control: RefCell::new(ProvisioningBearerControlHander::new()),
            transaction: RefCell::new(TransactionHandler::new()),
        }
    }

    pub(crate) fn next_transaction(&self) -> u8 {
        self.transaction_number
            .compare_exchange(0x7F, 0x00, Ordering::Acquire, Ordering::Relaxed);
        self.transaction_number.load(Ordering::Relaxed)
    }

    pub(crate) async fn tx_link_ack(&self, link_id: u32) -> Result<(), ()> {
        let ack = PDU {
            link_id: self
                .provisioning_bearer_control
                .borrow()
                .link_id
                .ok_or(())?,
            transaction_number: self.next_transaction(),
            pdu: GenericProvisioningPDU::ProvisioningBearerControl(
                ProvisioningBearerControl::LinkAck,
            ),
        };

        let mut xmit: Vec<u8, 128> = Vec::new();
        ack.emit(&mut xmit);
        self.tx.request(TxMessage::Transmit(&*xmit)).unwrap().await;
        Ok(())
    }

    pub(crate) async fn tx_transaction_ack(&self, transaction_number: u8) -> Result<(), ()> {
        let ack = PDU {
            link_id: self
                .provisioning_bearer_control
                .borrow()
                .link_id
                .ok_or(())?,
            transaction_number: self.transaction.borrow().transaction_number.ok_or(())?,
            pdu: GenericProvisioningPDU::TransactionAck,
        };

        let mut xmit: Vec<u8, 128> = Vec::new();
        ack.emit(&mut xmit);
        self.tx.request(TxMessage::Transmit(&*xmit)).unwrap().await;
        Ok(())
    }

    async fn handle_transaction_start(
        &mut self,
        link_id: u32,
        transaction_number: u8,
        tx_start: TransactionStart,
    ) {
        if matches!(self.state, State::Provisioning,) {
            self.state = State::Provisioning;

            let pdu = ProvisioningPDU::parse(&*tx_start.data).unwrap();
            defmt::info!("tx {}", pdu);

            match pdu {
                ProvisioningPDU::Invite(invite) => {
                    defmt::info!("INVITED sending capabilities");
                    self.state = State::Provisioning;

                    //let capabilities = PDU {
                    //link_id,
                    //transaction_number,
                    //pdu: ProvisioningPDU::Capabilities(self.capabilities.clone()),
                    //};

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
                                            let current_link_id =
                                                self.provisioning_bearer_control.borrow().link_id;
                                            if let Some(current_link_id) =
                                                self.provisioning_bearer_control.borrow().link_id
                                            {
                                                if current_link_id == pdu.link_id {
                                                    self.transaction
                                                        .borrow_mut()
                                                        .handle_transaction_start(
                                                            self,
                                                            pdu.transaction_number,
                                                            &tx_start,
                                                        )
                                                        .await;
                                                }
                                            }
                                        }
                                        GenericProvisioningPDU::TransactionContinuation {
                                            ..
                                        } => {
                                            defmt::info!("transaction continuation");
                                        }
                                        GenericProvisioningPDU::TransactionAck => {
                                            defmt::info!("transaction ack");
                                        }
                                        GenericProvisioningPDU::ProvisioningBearerControl(pbc) => {
                                            self.state = State::Provisioning;
                                            self.provisioning_bearer_control
                                                .borrow_mut()
                                                .handle(self, pdu.link_id, &pbc)
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
