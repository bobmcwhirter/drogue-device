use crate::drivers::ble::mesh::address::Address;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::driver::pipeline::provisioned::network::network_message_cache::NetworkMessageCache;
use crate::drivers::ble::mesh::pdu::network::{AuthenticatedPDU, ObfuscatedAndEncryptedPDU};

pub trait RelayContext {
    fn is_local_unicast(&self, address: &Address) -> bool;
}

pub struct Relay {
    cache: NetworkMessageCache,
}

impl Default for Relay {
    fn default() -> Self {
        Self {
            cache: Default::default(),
        }
    }
}

impl Relay {
    pub async fn process_inbound<C:RelayContext>(&mut self, ctx: &C, mut pdu: &AuthenticatedPDU) -> Result<Option<AuthenticatedPDU>, DeviceError> {
        if ! ctx.is_local_unicast(&pdu.dst) {
            if pdu.ttl >= 2 && ! self.cache.has_seen(pdu) {
                // decrease TTL and send a copy along.
                Ok(Some(AuthenticatedPDU {
                    ivi: pdu.ivi,
                    nid: pdu.nid,
                    ttl: pdu.ttl - 1,
                    seq: pdu.seq,
                    src: pdu.src,
                    dst: pdu.dst,
                    transport_pdu: pdu.transport_pdu.clone(),
                }))
            } else {
                // don't relay, TTL expired
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}