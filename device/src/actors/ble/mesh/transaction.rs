use core::marker::PhantomData;
use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::generic_provisioning::TransactionStart;
use crate::drivers::ble::mesh::transport::Transport;
use heapless::Vec;

pub(crate) struct TransactionHandler<T: Transport + 'static> {
    pub(crate) transaction_number: Option<u8>,
    segments: Option<Segments>,
    _marker: PhantomData<T>

}

impl<T: Transport + 'static> TransactionHandler<T> {

    pub(crate) fn new() -> Self {
        Self {
            transaction_number: None,
            segments: None,
            _marker: PhantomData
        }
    }

    pub(crate) async fn handle_transaction_start(&mut self, device: &Device<T>, transaction_number: u8, transaction_start: &TransactionStart) -> Result<(), ()>{
        self.transaction_number.replace(transaction_number);
        self.segments = Some(Segments::new(transaction_start.seg_n, &transaction_start.data));

        //self.seg_n.replace(transaction_start.seg_n);

        //if let Some(0) = self.seg_n {
            //device.tx_transaction_ack(transaction_number);
            //self.transaction_number.take();
            //self.seg_n.take();
        //}

        Ok(())
    }

    //pub(crate) async fn handle_transaction_continuation(&mut self, transaction_number: u32, transaction_continuation: &Tran) {

    //}

    pub(crate) async fn handle_transaction_ack() {
        // ignorable for this role
    }

}

struct Segments {
    segments: Vec<Option<Vec<u8,64>>, 32>,
}

impl Segments {
    fn new(seg_n: u8, data: &Vec<u8, 64>) -> Self {
        let mut this = Self {
            segments: Vec::new()
        };
        for n in 0..seg_n+1 {
            this.segments.push(None);
        }
        let mut chunk = Vec::new();
        chunk.extend_from_slice(data);
        this.segments[0] = Some(chunk);
        this
    }

    fn is_complete(&self) -> bool {
        self.segments.iter().all(|e| matches!(e, Some(_)))
    }
}