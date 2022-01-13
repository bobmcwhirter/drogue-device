use crate::actors::ble::mesh::bearer::{Tx, TxMessage};
use crate::actors::ble::mesh::transaction::{Transaction, TransactionMessage};
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::generic_provisioning::{
    GenericProvisioningPDU, ProvisioningBearerControl,
};
use crate::drivers::ble::mesh::transport::Transport;
use crate::{Actor, Address, Inbox};
use core::future::Future;
use heapless::Vec;

pub struct ProvisioningBearer<T>
where
    T: Transport + 'static,
{
    link_id: Option<u32>,
    up: Option<Address<Transaction<T>>>,
    down: Option<Address<Tx<T>>>,
}

impl<T> ProvisioningBearer<T>
where
    T: Transport + 'static,
{
    pub fn new() -> Self {
        Self {
            link_id: None,
            up: None,
            down: None,
        }
    }
}

pub enum ProvisioningBearerMessage<T>
where
    T: Transport + 'static,
{
    Initialize(Address<Tx<T>>, Address<Transaction<T>>),
    Inbound(PDU),
    Outbound(u8, GenericProvisioningPDU),
}

impl<T> Actor for ProvisioningBearer<T>
where
    T: Transport + 'static,
{
    type Message<'m> = ProvisioningBearerMessage<T>;
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
                let message = inbox.next().await;

                if let Some(mut message) = message {
                    let message = message.message();
                    match message {
                        ProvisioningBearerMessage::Initialize(ref down, ref up) => {
                            self.down.replace(*down);
                            self.up.replace(*up);
                        }

                        ProvisioningBearerMessage::Inbound(pdu) => {
                            match &pdu.pdu {
                                GenericProvisioningPDU::ProvisioningBearerControl(pbc) => {
                                    match pbc {
                                        ProvisioningBearerControl::LinkOpen(uuid) => {
                                            // TODO 60-second timer
                                            if let Some(link_id) = self.link_id {
                                                if link_id == pdu.link_id {
                                                    // same link, ACK it.
                                                } else {
                                                    // new link, did we miss a close?
                                                    self.link_id.replace(pdu.link_id);
                                                }
                                            } else {
                                                // started new link
                                                self.link_id.replace(pdu.link_id);
                                            }
                                        }
                                        ProvisioningBearerControl::LinkAck => {}
                                        ProvisioningBearerControl::LinkClose(reason) => {
                                            self.link_id.take();
                                        }
                                    }
                                }
                                _ => {
                                    if let Some(link_id) = self.link_id {
                                        // Further processing only occurs if the link_id is correct.
                                        if link_id == pdu.link_id {
                                            self.up.unwrap().request(TransactionMessage::Inbound(pdu.clone())).unwrap().await;
                                        }
                                    }
                                }
                            }
                        }

                        ProvisioningBearerMessage::Outbound(transaction_number, pdu) => {
                            // transmit outwards.
                            if let Some(link_id) = self.link_id {
                                let pdu = PDU {
                                    link_id,
                                    transaction_number: *transaction_number,
                                    pdu: pdu.clone(),
                                };
                                let mut xmit: Vec<u8, 128> = Vec::new();
                                pdu.emit(&mut xmit);
                                self.down.unwrap().request(TxMessage::Transmit(&*xmit)).unwrap().await;
                            }
                        }
                    }
                }
            }
        }
    }
}
