use heapless::Vec;
use rand_core::{CryptoRng, RngCore};
use core::future::Future;
use core::borrow::BorrowMut;
use aes::Aes128;
use cmac::Cmac;
use cmac::crypto_mac::Output;
use p256::PublicKey;
use crate::drivers::ble::mesh::driver::pipeline::mesh::MeshContext;
use crate::drivers::ble::mesh::driver::pipeline::PipelineContext;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::driver::DeviceError;
use crate::drivers::ble::mesh::driver::node::{Node, Receiver, Transmitter};
use crate::drivers::ble::mesh::driver::pipeline::provisionable::ProvisionableContext;
use crate::drivers::ble::mesh::generic_provisioning::GenericProvisioningPDU;
use crate::drivers::ble::mesh::provisioning::ProvisioningData;
use crate::drivers::ble::mesh::vault::Vault;

impl<TX, RX, V, R> ProvisionableContext for Node<TX, RX, V, R>
    where
        TX: Transmitter,
        RX: Receiver,
        V: Vault,
        R: RngCore + CryptoRng,
{
    fn rng_fill(&self, dest: &mut [u8]) {
        self.rng.borrow_mut().fill_bytes(dest);
    }

    type SetPeerPublicKeyFuture<'m>
        where
            Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn set_peer_public_key<'m>(&self, pk: PublicKey) -> Self::SetPeerPublicKeyFuture<'m> {
        self.vault.borrow_mut().set_peer_public_key(pk)
    }

    fn public_key(&self) -> Result<PublicKey, DeviceError> {
        self.vault.borrow().public_key()
    }

    type SetProvisioningDataFuture<'m>
        where
            Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn set_provisioning_data<'m>(
        &self,
        data: &'m ProvisioningData,
    ) -> Self::SetProvisioningDataFuture<'m> {
        self.vault.borrow_mut().set_provisioning_data(data)
    }

    fn aes_cmac(&self, key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.borrow().aes_cmac(key, input)
    }

    fn s1(&self, input: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.borrow().s1(input)
    }

    fn prsk(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.borrow().prsk(salt)
    }

    fn prsn(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.borrow().prsn(salt)
    }

    fn prck(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.borrow().prck(salt)
    }

    fn aes_ccm_decrypt(&self, key: &[u8], nonce: &[u8], data: &mut [u8], mic: &[u8]) -> Result<(), DeviceError> {
        self.vault.borrow().aes_ccm_decrypt(key, nonce, data, mic)
    }

    fn rng_u8(&self) -> u8 {
        (self.rng.borrow_mut().next_u32() & 0xFF) as u8
    }

    fn rng_u32(&self) -> u32 {
        self.rng.borrow_mut().next_u32()
    }
}

impl<TX, RX, V, R> MeshContext for Node<TX, RX, V, R>
    where
        TX: Transmitter,
        RX: Receiver,
        V: Vault,
        R: RngCore + CryptoRng,
{
    fn uuid(&self) -> Uuid {
        self.vault.borrow().uuid()
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

impl<TX, RX, V, R> PipelineContext for Node<TX, RX, V, R>
    where
        TX: Transmitter,
        RX: Receiver,
        V: Vault,
        R: RngCore + CryptoRng,
{
}
