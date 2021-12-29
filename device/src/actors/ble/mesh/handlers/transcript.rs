use aes::Aes128;
use cmac::Cmac;
use cmac::crypto_mac::Output;
use crate::drivers::ble::mesh::provisioning::{
    Capabilities, Invite, PublicKey, Start,
};
use heapless::Vec;
use crate::drivers::ble::mesh::crypto::s1;

pub struct Transcript {
    // TODO improve the size of this vec
    confirmation_inputs: Vec<u8, 256>,
}

impl Transcript {
    pub fn new() -> Self {
        Self { confirmation_inputs: Vec::new() }
    }

    pub(crate) fn add_invite(&mut self, invite: &Invite) -> Result<(), ()> {
        let mut vec: Vec<u8, 2> = Vec::new();
        invite.emit(&mut vec)?;
        self.confirmation_inputs.extend_from_slice(&vec.as_slice()[1..])
    }

    pub(crate) fn add_capabilities(&mut self, capabilities: &Capabilities) -> Result<(), ()> {
        let mut vec: Vec<u8, 32> = Vec::new();
        capabilities.emit(&mut vec)?;
        self.confirmation_inputs.extend_from_slice(&vec.as_slice()[1..])
    }

    pub(crate) fn add_start(&mut self, start: &Start) -> Result<(), ()> {
        let mut vec: Vec<u8, 32> = Vec::new();
        start.emit(&mut vec)?;
        self.confirmation_inputs.extend_from_slice(&vec.as_slice()[1..])
    }

    pub(crate) fn add_pubkey_provisioner(&mut self, pk: &PublicKey) -> Result<(), ()> {
        let mut vec: Vec<u8, 65> = Vec::new();
        pk.emit(&mut vec)?;
        self.confirmation_inputs.extend_from_slice(&vec.as_slice()[1..])
    }

    pub(crate) fn add_pubkey_device(&mut self, pk: &PublicKey) -> Result<(), ()> {
        let mut vec: Vec<u8, 65> = Vec::new();
        pk.emit(&mut vec)?;
        self.confirmation_inputs.extend_from_slice(&vec.as_slice()[1..])
    }

    fn confirmation_inputs(&self) -> &[u8] {
        self.confirmation_inputs.as_slice()
    }

    pub(crate) fn confirmation_salt(&self) -> Result<Output<Cmac<Aes128>>,()> {
        s1(self.confirmation_inputs())
    }
}
