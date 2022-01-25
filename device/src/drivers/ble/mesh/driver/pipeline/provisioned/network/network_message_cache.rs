use crate::drivers::ble::mesh::pdu::network::AuthenticatedPDU;

pub struct NetworkMessageCache {

}

impl Default for NetworkMessageCache {
    fn default() -> Self {
        Self {

        }
    }
}

impl NetworkMessageCache {

    pub fn has_seen(&mut self, pdu: &AuthenticatedPDU) -> bool {
        todo!()
    }

}