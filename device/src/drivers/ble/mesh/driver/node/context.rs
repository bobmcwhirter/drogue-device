use crate::drivers::ble::mesh::address::Address;
use crate::drivers::ble::mesh::configuration_manager::{KeyStorage, NetworkKey};
use crate::drivers::ble::mesh::crypto;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::driver::node::{Node, Receiver, Transmitter};
use crate::drivers::ble::mesh::driver::pipeline::mesh::MeshContext;
use crate::drivers::ble::mesh::driver::pipeline::provisioned::access::AccessContext;
use crate::drivers::ble::mesh::driver::pipeline::provisioned::lower::LowerContext;
use crate::drivers::ble::mesh::driver::pipeline::provisioned::network::authentication::{
    AuthenticationContext, AuthenticationOutput,
};
use crate::drivers::ble::mesh::driver::pipeline::provisioned::network::relay::RelayContext;
use crate::drivers::ble::mesh::driver::pipeline::provisioned::upper::UpperContext;
use crate::drivers::ble::mesh::driver::pipeline::provisioned::ProvisionedContext;
use crate::drivers::ble::mesh::driver::pipeline::unprovisioned::provisionable::UnprovisionedContext;
use crate::drivers::ble::mesh::driver::pipeline::PipelineContext;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::pdu::bearer::advertising::PDU;
use crate::drivers::ble::mesh::pdu::network::ObfuscatedAndEncryptedPDU;
use crate::drivers::ble::mesh::provisioning::ProvisioningData;
use crate::drivers::ble::mesh::storage::Storage;
use crate::drivers::ble::mesh::vault::Vault;
use aes::Aes128;
use cmac::crypto_mac::Output;
use cmac::Cmac;
use core::future::Future;
use heapless::Vec;
use p256::PublicKey;
use rand_core::{CryptoRng, RngCore};

// ------------------------------------------------------------------------
// Unprovisioned pipeline context
// ------------------------------------------------------------------------

impl<TX, RX, S, R> UnprovisionedContext for Node<TX, RX, S, R>
where
    TX: Transmitter,
    RX: Receiver,
    S: Storage,
    R: RngCore + CryptoRng,
{
    fn rng_fill(&self, dest: &mut [u8]) {
        self.rng.borrow_mut().fill_bytes(dest);
    }

    type SetPeerPublicKeyFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn set_peer_public_key<'m>(&'m self, pk: PublicKey) -> Self::SetPeerPublicKeyFuture<'m> {
        async move { self.vault().set_peer_public_key(pk).await }
    }

    fn public_key(&self) -> Result<PublicKey, DeviceError> {
        self.vault().public_key()
    }

    type SetProvisioningDataFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn set_provisioning_data<'m>(
        &'m self,
        data: &'m ProvisioningData,
    ) -> Self::SetProvisioningDataFuture<'m> {
        async move { self.vault().set_provisioning_data(data).await }
    }

    fn aes_cmac(&self, key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault().aes_cmac(key, input)
    }

    fn s1(&self, input: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        crypto::s1(input).map_err(|_| DeviceError::InvalidKeyLength)
    }

    fn prsk(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault().prsk(salt)
    }

    fn prsn(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault().prsn(salt)
    }

    fn prck(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault().prck(salt)
    }

    fn aes_ccm_decrypt(
        &self,
        key: &[u8],
        nonce: &[u8],
        data: &mut [u8],
        mic: &[u8],
    ) -> Result<(), DeviceError> {
        crypto::aes_ccm_decrypt_detached(key, nonce, data, mic)
            .map_err(|_| DeviceError::CryptoError)
    }

    fn rng_u8(&self) -> u8 {
        (self.rng.borrow_mut().next_u32() & 0xFF) as u8
    }

    fn rng_u32(&self) -> u32 {
        self.rng.borrow_mut().next_u32()
    }
}

impl<TX, RX, S, R> MeshContext for Node<TX, RX, S, R>
where
    TX: Transmitter,
    RX: Receiver,
    S: Storage,
    R: RngCore + CryptoRng,
{
    fn uuid(&self) -> Uuid {
        self.vault().uuid()
    }

    type TransmitFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn transmit_pdu<'m>(&'m self, pdu: PDU) -> Self::TransmitFuture<'m> {
        async move {
            let mut bytes = Vec::<u8, 64>::new();
            pdu.emit(&mut bytes)
                .map_err(|_| DeviceError::InsufficientBuffer)?;
            self.transmitter.transmit_bytes(&*bytes).await
        }
    }
}

// ------------------------------------------------------------------------
// Provisioned pipeline context
// ------------------------------------------------------------------------

impl<TX, RX, S, R> ProvisionedContext for Node<TX, RX, S, R>
where
    R: CryptoRng + RngCore,
    RX: Receiver,
    S: Storage,
    TX: Transmitter,
{
}

impl<TX, RX, S, R> RelayContext for Node<TX, RX, S, R>
where
    R: CryptoRng + RngCore,
    RX: Receiver,
    S: Storage,
    TX: Transmitter,
{
    fn is_local_unicast(&self, address: &Address) -> bool {
        todo!()
    }
}

impl<TX, RX, S, R> AuthenticationContext for Node<TX, RX, S, R>
where
    R: CryptoRng + RngCore,
    RX: Receiver,
    S: Storage,
    TX: Transmitter,
{
    fn iv_index(&self) -> Option<u32> {
        self.vault().iv_index()
    }

    fn network_keys(&self, nid: u8) -> Vec<NetworkKey, 10> {
        self.vault().network_keys(nid)
    }
}

impl<TX, RX, S, R> LowerContext for Node<TX, RX, S, R>
where
    R: CryptoRng + RngCore,
    RX: Receiver,
    S: Storage,
    TX: Transmitter,
{
}

impl<TX, RX, S, R> UpperContext for Node<TX, RX, S, R>
where
    R: CryptoRng + RngCore,
    RX: Receiver,
    S: Storage,
    TX: Transmitter,
{
}

impl<TX, RX, S, R> AccessContext for Node<TX, RX, S, R>
where
    R: CryptoRng + RngCore,
    RX: Receiver,
    S: Storage,
    TX: Transmitter,
{
}

impl<TX, RX, S, R> PipelineContext for Node<TX, RX, S, R>
where
    TX: Transmitter,
    RX: Receiver,
    S: Storage,
    R: RngCore + CryptoRng,
{
}
