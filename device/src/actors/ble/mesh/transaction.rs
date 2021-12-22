use core::marker::PhantomData;
use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::generic_provisioning::TransactionStart;
use crate::drivers::ble::mesh::transport::Transport;

pub(crate) struct TransactionHandler<T: Transport + 'static> {
    pub(crate) transaction_number: Option<u8>,
    _marker: PhantomData<T>

}

impl<T: Transport + 'static> TransactionHandler<T> {

    pub(crate) fn new() -> Self {
        Self {
            transaction_number: None,
            _marker: PhantomData
        }
    }

    pub(crate) async fn handle_transaction_start(&mut self, device: &Device<T>, transaction_number: u8, transaction_start: &TransactionStart) {
        device.tx_transaction_ack(transaction_number);

    }

    //pub(crate) async fn handle_transaction_continuation(&mut self, transaction_number: u32, transaction_continuation: &Tran) {

    //}

    pub(crate) async fn handle_transaction_ack() {
        // ignorable for this role
    }

}