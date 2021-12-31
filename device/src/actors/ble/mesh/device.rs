use super::handlers::ProvisioningBearerControlHander;
use super::handlers::TransactionHandler;
use crate::actors::ble::mesh::bearer::{Tx, TxMessage};
use crate::actors::ble::mesh::configuration_manager::{ConfigurationManager, KeyStorage, Keys};
use crate::actors::ble::mesh::handlers::ProvisioningHandler;
use crate::actors::ble::mesh::key_manager::KeyManager;
use crate::drivers::ble::mesh::bearer::advertising;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::generic_provisioning::{
    GenericProvisioningPDU, ProvisioningBearerControl,
};
use crate::drivers::ble::mesh::provisioning::{Capabilities, ParseError, ProvisioningPDU};
use crate::drivers::ble::mesh::storage::Storage;
use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::drivers::ble::mesh::{InsufficientBuffer, MESH_MESSAGE, PB_ADV};
use crate::{Actor, Address, Inbox};
use cmac::crypto_mac::InvalidKeyLength;
use core::cell::RefCell;
use core::cell::RefMut;
use core::future::Future;
use core::sync::atomic::{AtomicU8, Ordering};
use defmt::Format;
use embassy::time::{Duration, Ticker};
use futures::future::{select, Either};
use futures::{pin_mut, StreamExt};
use heapless::Vec;
use p256::PublicKey;
use rand_core::{CryptoRng, RngCore};

enum State {
    Unprovisioned,
    Provisioning,
    Provisioned,
}

#[derive(Format)]
pub enum DeviceError {
    NoServices,
    StorageInitialization,
    KeyInitialization,
    InvalidPacket,
    InsufficientBuffer,
    InvalidLink,
    NoEstablishedLink,
    InvalidKeyLength,
    InvalidTransactionNumber,
    IncompleteTransaction,
    NoSharedSecret,
    ParseError(ParseError),
}

impl From<InsufficientBuffer> for DeviceError {
    fn from(_: InsufficientBuffer) -> Self {
        Self::InsufficientBuffer
    }
}

impl From<InvalidKeyLength> for DeviceError {
    fn from(_: InvalidKeyLength) -> Self {
        Self::InvalidKeyLength
    }
}

impl From<ParseError> for DeviceError {
    fn from(inner: ParseError) -> Self {
        Self::ParseError(inner)
    }
}

pub struct Device<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    pub(crate) uuid: Uuid,
    pub(crate) capabilities: Capabilities,
    transaction_number: AtomicU8,
    state: State,
    tx: Address<Tx<T>>,
    ticker: Ticker,
    // Crypto
    rng: RefCell<R>,
    config_manager: ConfigurationManager<S>,
    pub(crate) key_manager: RefCell<KeyManager<R, Device<T, R, S>>>,
    // Transport
    outbound: RefCell<Option<ProvisioningPDU>>,
    // Handlers
    provisioning_bearer_control: RefCell<ProvisioningBearerControlHander<T, R, S>>,
    transaction: RefCell<TransactionHandler<T, R, S>>,
    provisioning: RefCell<ProvisioningHandler<T, R, S>>,
}

impl<T, R, S> Device<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    pub fn new(
        mut rng: R,
        storage: S,
        uuid: Uuid,
        capabilities: Capabilities,
        tx: Address<Tx<T>>,
    ) -> Self {
        let key_manager = KeyManager::new();
        let provisioning = ProvisioningHandler::new(&mut rng);
        Self {
            uuid,
            capabilities,
            transaction_number: AtomicU8::new(0x80),
            state: State::Unprovisioned,
            tx,
            ticker: Ticker::every(Duration::from_secs(3)),
            config_manager: ConfigurationManager::new(storage),
            key_manager: RefCell::new(key_manager),
            rng: RefCell::new(rng),
            outbound: RefCell::new(None),
            provisioning_bearer_control: RefCell::new(ProvisioningBearerControlHander::new()),
            transaction: RefCell::new(TransactionHandler::new()),
            provisioning: RefCell::new(provisioning),
        }
    }

    async fn initialize(&mut self) -> Result<(), DeviceError> {
        defmt::trace!("** initializing config_manager");
        self.config_manager
            .initialize()
            .await
            .map_err(|_| DeviceError::StorageInitialization)?;
        defmt::trace!("   complete");
        defmt::trace!("** initializing key_manager");
        self.key_manager
            .borrow_mut()
            .initialize(self as *const _)
            .await?;
        defmt::trace!("   complete");
        Ok(())
    }

    pub(crate) fn public_key(&self) -> Result<PublicKey, DeviceError> {
        self.key_manager.borrow().public_key()
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
        let mut bytes = [0; 1];
        self.rng.borrow_mut().fill_bytes(&mut bytes);
        bytes[0]
    }

    pub(crate) fn link_id(&self) -> Option<u32> {
        self.provisioning_bearer_control.borrow().link_id
    }

    pub(crate) async fn tx(&self, data: &[u8]) -> Result<(), ()> {
        // not actually infallible but...
        self.tx
            .request(TxMessage::Transmit(data))
            .map_err(|_| ())?
            .await;
        Ok(())
    }

    pub(crate) async fn tx_pdu(&self, pdu: PDU) -> Result<(), DeviceError> {
        let mut xmit: Vec<u8, 128> = Vec::new();
        pdu.emit(&mut xmit)?;
        self.tx(&*xmit).await;
        Ok(())
    }

    pub(crate) async fn tx_link_ack(&self, link_id: u32) -> Result<(), DeviceError> {
        defmt::trace!("<< LinkAck({})", link_id);
        self.tx_pdu(PDU {
            link_id: link_id,
            transaction_number: self.next_transaction(),
            pdu: GenericProvisioningPDU::ProvisioningBearerControl(
                ProvisioningBearerControl::LinkAck,
            ),
        })
        .await
    }

    pub(crate) async fn tx_transaction_ack(
        &self,
        transaction_number: u8,
    ) -> Result<(), DeviceError> {
        if let Some(link_id) = self.link_id() {
            defmt::trace!("<< TransactionAck({})", transaction_number);
            self.tx_pdu(PDU {
                link_id,
                transaction_number,
                pdu: GenericProvisioningPDU::TransactionAck,
            })
            .await
        } else {
            Ok(())
        }
    }

    pub(crate) fn tx_provisioning_pdu(&self, pdu: ProvisioningPDU) {
        self.outbound.borrow_mut().replace(pdu);
    }

    pub(crate) fn tx_capabilities(&self) {
        defmt::trace!("<< Capabilities");
        let pdu = ProvisioningPDU::Capabilities(self.capabilities.clone());
        self.outbound.borrow_mut().replace(pdu);
    }

    pub(crate) async fn handle_provisioning_pdu(
        &self,
        pdu: ProvisioningPDU,
    ) -> Result<(), DeviceError> {
        self.provisioning.borrow_mut().handle(self, pdu).await?;
        Ok(())
    }

    pub(crate) async fn handle_transmit(&self) -> Result<(), DeviceError> {
        let mut outbound = self.outbound.borrow_mut();
        self.transaction
            .borrow_mut()
            .handle_outbound(self, outbound.take())
            .await?;
        Ok(())
    }

    async fn handle_pdu(&mut self, pdu: PDU) -> Result<(), DeviceError> {
        if let GenericProvisioningPDU::ProvisioningBearerControl(_) = pdu.pdu {
            // we'll delegate link_id checking to the PBC handler.
        } else {
            match self.provisioning_bearer_control.borrow().link_id {
                None => {
                    return Err(DeviceError::NoEstablishedLink);
                }
                Some(current_link_id) => {
                    if current_link_id != pdu.link_id {
                        return Err(DeviceError::InvalidLink);
                    }
                }
            }
        }

        match pdu.pdu {
            GenericProvisioningPDU::TransactionStart(tx_start) => {
                self.transaction
                    .borrow_mut()
                    .handle_transaction_start(self, pdu.transaction_number, &tx_start)
                    .await
            }
            GenericProvisioningPDU::TransactionContinuation(tx_cont) => {
                self.transaction
                    .borrow_mut()
                    .handle_transaction_continuation(self, pdu.transaction_number, &tx_cont)
                    .await
            }
            GenericProvisioningPDU::TransactionAck => {
                self.transaction
                    .borrow_mut()
                    .handle_transaction_ack(self, pdu.transaction_number)
                    .await
            }
            GenericProvisioningPDU::ProvisioningBearerControl(pbc) => {
                self.state = State::Provisioning;
                self.provisioning_bearer_control
                    .borrow_mut()
                    .handle(self, pdu.link_id, &pbc)
                    .await
            }
        }
    }

    pub(crate) fn link_close(&self) {
        self.transaction.borrow_mut().link_close();
        self.provisioning.borrow_mut().reset();
    }
}

impl<T, R, S> Handler for Address<Device<T, R, S>>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    fn handle<'m>(&self, message: Vec<u8, 384>) {
        self.notify(message).ok();
    }
}

impl<T, R, S> Actor for Device<T, R, S>
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
                let inbox_fut = inbox.next();
                let ticker_fut = self.ticker.next();

                pin_mut!(inbox_fut);
                pin_mut!(ticker_fut);

                let result = match select(inbox_fut, ticker_fut).await {
                    Either::Left((ref mut msg, _)) => match msg {
                        Some(message) => {
                            let data = message.message();
                            if data.len() >= 2 {
                                if data[1] == PB_ADV {
                                    let pdu = advertising::PDU::parse(data);
                                    if let Ok(pdu) = pdu {
                                        self.handle_pdu(pdu).await
                                    } else {
                                        Err(DeviceError::InvalidPacket)
                                    }
                                } else if data[1] == MESH_MESSAGE {
                                    defmt::info!("saw mesh message {:x}", data);
                                    Ok(())
                                } else {
                                    defmt::info!("saw unknown message {:x}", data);
                                    Ok(())
                                }
                            } else {
                                // Not long enough to bother with.
                                Err(DeviceError::InvalidPacket)
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

impl<T, R, S> KeyStorage for Device<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    type StoreFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), ()>>;

    fn store<'m>(&'m self, keys: Keys) -> Self::StoreFuture<'m> {
        self.config_manager.store(keys)
    }

    fn retrieve<'m>(&'m self) -> Keys {
        self.config_manager.retrieve()
    }
}

pub trait RandomProvider<R: RngCore + CryptoRng> {
    fn rng(&self) -> RefMut<'_, R>;
}

impl<T, R, S> RandomProvider<R> for Device<T, R, S>
where
    T: Transport + 'static,
    R: RngCore + CryptoRng + 'static,
    S: Storage + 'static,
{
    fn rng(&self) -> RefMut<'_, R> {
        self.rng.borrow_mut()
    }
}
