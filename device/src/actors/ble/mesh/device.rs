use super::handlers::ProvisioningBearerControlHander;
use super::handlers::TransactionHandler;
use crate::actors::ble::mesh::bearer::{Tx, TxMessage};
use crate::actors::ble::mesh::handlers::ProvisioningHandler;
use crate::actors::ble::mesh::key_manager::KeyManager;
use crate::drivers::ble::mesh::bearer::advertising;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::generic_provisioning::{
    GenericProvisioningPDU, ProvisioningBearerControl
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
use p256::PublicKey;
use rand_core::RngCore;

enum State {
    Unprovisioned,
    Provisioning,
    Provisioned,
}

pub struct Device<T, R>
where
    T: Transport + 'static,
    R: RngCore + 'static,
{
    pub(crate) uuid: Uuid,
    pub(crate) capabilities: Capabilities,
    transaction_number: AtomicU8,
    state: State,
    tx: Address<Tx<T>>,
    ticker: Ticker,
    // Crypto
    rng: RefCell<R>,
    pub(crate) key_manager: KeyManager,
    // Transport
    outbound: RefCell<Option<ProvisioningPDU>>,
    // Handlers
    provisioning_bearer_control: RefCell<ProvisioningBearerControlHander<T, R>>,
    transaction: RefCell<TransactionHandler<T, R>>,
    provisioning: RefCell<ProvisioningHandler<T, R>>,
}

impl<T, R> Device<T, R>
where
    T: Transport + 'static,
    R: RngCore + 'static,
{
    pub fn new(mut rng: R, uuid: Uuid, capabilities: Capabilities, tx: Address<Tx<T>>) -> Self {
        let key_manager = KeyManager::new(&mut rng);
        Self {
            uuid,
            capabilities,
            transaction_number: AtomicU8::new(0x80),
            state: State::Unprovisioned,
            tx,
            ticker: Ticker::every(Duration::from_secs(3)),
            key_manager,
            rng: RefCell::new(rng),
            outbound: RefCell::new(None),
            provisioning_bearer_control: RefCell::new(ProvisioningBearerControlHander::new()),
            transaction: RefCell::new(TransactionHandler::new()),
            provisioning: RefCell::new(ProvisioningHandler::new()),
        }
    }

    pub(crate) fn public_key(&self) -> PublicKey {
        self.key_manager.public_key()
    }

    pub(crate) fn next_transaction(&self) -> u8 {
        let num = self.transaction_number.load(Ordering::SeqCst);
        self.transaction_number.fetch_add(1, Ordering::SeqCst);
        num
    }

    pub(crate) fn next_random_u32(&self) -> u32 {
        self.rng.borrow_mut().next_u32()
    }

    pub(crate) fn next_random_u8(&self) -> u8 {
        let mut bytes = [0;1];
        self.rng.borrow_mut().fill_bytes(&mut bytes);
        bytes[0]
    }

    pub(crate) fn link_id(&self) -> Result<u32, ()> {
        self.provisioning_bearer_control.borrow().link_id.ok_or(())
    }

    pub(crate) async fn tx(&self, data: &[u8]) -> Result<(), ()> {
        self.tx.request(TxMessage::Transmit(data)).map_err(|_|())?.await;
        Ok(())
    }

    pub(crate) async fn tx_pdu(&self, pdu: PDU) -> Result<(), ()> {
        defmt::info!("<< outbound << {}", pdu);
        let mut xmit: Vec<u8, 128> = Vec::new();
        pdu.emit(&mut xmit)?;
        self.tx(&*xmit).await
    }

    pub(crate) async fn tx_link_ack(&self, link_id: u32) -> Result<(), ()> {
        self.tx_pdu(PDU {
            link_id: link_id,
            transaction_number: self.next_transaction(),
            pdu: GenericProvisioningPDU::ProvisioningBearerControl(
                ProvisioningBearerControl::LinkAck,
            ),
        })
        .await
    }

    pub(crate) async fn tx_transaction_ack(&self, transaction_number: u8) -> Result<(), ()> {
        self.tx_pdu(PDU {
            link_id: self.link_id()?,
            transaction_number,
            pdu: GenericProvisioningPDU::TransactionAck,
        })
        .await
    }

    pub(crate) fn tx_provisioning_pdu(&self, pdu: ProvisioningPDU) {
        defmt::info!("NEXT OUTBOUND {}", pdu);
        self.outbound.borrow_mut().replace(pdu);
    }

    pub(crate) fn tx_capabilities(&self) -> Result<(), ()> {
        let pdu = ProvisioningPDU::Capabilities(self.capabilities.clone());
        self.outbound.borrow_mut().replace(pdu);
        Ok(())
    }

    pub(crate) async fn handle_provisioning_pdu(&self, pdu: ProvisioningPDU) -> Result<(), ()> {
        defmt::info!("handle_provisioning_pdu: {}", pdu);
        self.provisioning.borrow_mut().handle(self, pdu).await?;
        Ok(())
    }

    pub(crate) async fn handle_transmit(&self) -> Result<(), ()> {
        let mut outbound = self.outbound.borrow_mut();
        self.transaction
            .borrow_mut()
            .handle_outbound(self, outbound.take())
            .await?;
        Ok(())
    }
}

impl<T, R> Handler for Address<Device<T, R>>
where
    T: Transport + 'static,
    R: RngCore + 'static,
{
    fn handle<'m>(&self, message: Vec<u8, 384>) {
        self.notify(message).ok();
    }
}

impl<T, R> Actor for Device<T, R>
where
    T: Transport + 'static,
    R: RngCore + 'static,
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
                let inbox_fut = inbox.next();
                let ticker_fut = self.ticker.next();

                pin_mut!(inbox_fut);
                pin_mut!(ticker_fut);

                let result = match select(inbox_fut, ticker_fut).await {
                    Either::Left((ref mut msg, _)) => match msg {
                        Some(message) => {
                            let data = message.message();
                            if data.len() >= 2 && data[1] == PB_ADV {
                                let pdu = advertising::PDU::parse(data);
                                defmt::info!(">> inbound >> {}", pdu);

                                if let Ok(pdu) = pdu {
                                    match pdu.pdu {
                                        GenericProvisioningPDU::TransactionStart(tx_start) => {
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
                                                        .await
                                                } else {
                                                    Ok(())
                                                }
                                            } else {
                                                Ok(())
                                            }
                                        }
                                        GenericProvisioningPDU::TransactionContinuation(
                                            tx_cont,
                                        ) => {
                                            if let Some(current_link_id) =
                                                self.provisioning_bearer_control.borrow().link_id
                                            {
                                                if current_link_id == pdu.link_id {
                                                    self.transaction
                                                        .borrow_mut()
                                                        .handle_transaction_continuation(
                                                            self,
                                                            pdu.transaction_number,
                                                            &tx_cont,
                                                        )
                                                        .await
                                                } else {
                                                    Ok(())
                                                }
                                            } else {
                                                Ok(())
                                            }
                                        }
                                        GenericProvisioningPDU::TransactionAck => {
                                            if let Some(current_link_id) =
                                                self.provisioning_bearer_control.borrow().link_id
                                            {
                                                if current_link_id == pdu.link_id {
                                                    self.transaction
                                                        .borrow_mut()
                                                        .handle_transaction_ack(
                                                            self,
                                                            pdu.transaction_number,
                                                        )
                                                        .await
                                                } else {
                                                    Ok(())
                                                }
                                            } else {
                                                Ok(())
                                            }
                                        }
                                        GenericProvisioningPDU::ProvisioningBearerControl(pbc) => {
                                            self.state = State::Provisioning;
                                            self.provisioning_bearer_control
                                                .borrow_mut()
                                                .handle(self, pdu.link_id, &pbc)
                                                .await
                                        }
                                    }
                                } else {
                                    Ok(())
                                }
                            } else {
                                // Not a PB-ADV
                                Ok(())
                            }
                        }
                        _ => {
                            // ignore
                            Ok(())
                        }
                    },
                    Either::Right((_, _)) => {
                        if matches!(self.state, State::Unprovisioned) {
                            self.tx
                                .request(TxMessage::UnprovisionedBeacon(self.uuid))
                                .unwrap()
                                .await;
                            Ok(())
                        } else {
                            Ok(())
                        }
                    }
                };
                if let Err(err) = result {
                    defmt::error!("BLE-Mesh error: {}", err);
                }
                if let Err(err) = self.handle_transmit().await {
                    defmt::error!("BLE-Mesh error: {}", err);
                }
            }
        }
    }
}
