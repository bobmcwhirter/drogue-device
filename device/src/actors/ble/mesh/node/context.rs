use heapless::Vec;
use rand_core::{CryptoRng, RngCore};
use crate::actors::ble::mesh::node::{Node, Transmitter};
use crate::actors::ble::mesh::pipeline::provisionable::ProvisionableContext;
use crate::actors::ble::mesh::vault::Vault;
use core::future::Future;
use aes::Aes128;
use cmac::Cmac;
use cmac::crypto_mac::Output;
use p256::PublicKey;
use crate::actors::ble::mesh::device::DeviceError;
use crate::actors::ble::mesh::pipeline::mesh::MeshContext;
use crate::actors::ble::mesh::pipeline::PipelineContext;
use crate::actors::ble::mesh::pipeline::provisioning_bearer::ProvisioningBearerContext;
use crate::actors::ble::mesh::pipeline::transaction::TransactionContext;
use crate::drivers::ble::mesh::bearer::advertising::PDU;
use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::generic_provisioning::GenericProvisioningPDU;
use crate::drivers::ble::mesh::provisioning::ProvisioningData;

impl<T, V, R> ProvisionableContext for Node<T, V, R>
    where
        T: Transmitter,
        V: Vault,
        R: RngCore + CryptoRng,
{
    fn rng_fill(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }

    type SetPeerPublicKeyFuture<'m>
        where
            Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn set_peer_public_key<'m>(&mut self, pk: PublicKey) -> Self::SetPeerPublicKeyFuture<'m> {
        self.vault.set_peer_public_key(pk)
    }

    fn public_key(&self) -> Result<PublicKey, DeviceError> {
        self.vault.public_key()
    }

    type SetProvisioningDataFuture<'m>
        where
            Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn set_provisioning_data<'m>(
        &mut self,
        data: &'m ProvisioningData,
    ) -> Self::SetProvisioningDataFuture<'m> {
        self.vault.set_provisioning_data(data)
    }

    fn aes_cmac(&self, key: &[u8], input: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.aes_cmac(key, input)
    }

    fn s1(&self, input: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.s1(input)
    }

    fn prsk(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.prsk(salt)
    }

    fn prsn(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.prsn(salt)
    }

    fn prck(&self, salt: &[u8]) -> Result<Output<Cmac<Aes128>>, DeviceError> {
        self.vault.prck(salt)
    }

    fn aes_ccm_decrypt(&self, key: &[u8], nonce: &[u8], data: &mut [u8], mic: &[u8]) -> Result<(), DeviceError> {
        self.vault.aes_ccm_decrypt(key, nonce, data, mic)
    }

    fn rng_u8(&mut self) -> u8 {
        (self.rng.next_u32() & 0xFF) as u8
    }

    fn rng_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }
}

impl<T, V, R> TransactionContext for Node<T, V, R>
    where
        T: Transmitter,
        V: Vault,
        R: RngCore + CryptoRng,
{
    type GenericProvisioningPduFuture<'m>
        where
            Self: 'm,
    = impl Future<Output = Result<(), DeviceError>>;

    fn transmit_generic_provisioning_pdu<'m>(&'m self, pdu: &'m GenericProvisioningPDU) -> Self::GenericProvisioningPduFuture<'m> {
        async move {
            match (self.link_id, self.transaction_number) {
                (Some(link_id), Some(transaction_number)) => {
                    self.transmit_pdu(PDU {
                        link_id,
                        transaction_number,
                        pdu: GenericProvisioningPDU::TransactionAck,
                    }).await
                }
                (None, _) => {
                    Err(DeviceError::InvalidLink)
                }
                (_, None) => {
                    Err(DeviceError::InvalidTransactionNumber)
                }
            }
        }
    }
}

impl<T, V, R> ProvisioningBearerContext for Node<T, V, R>
    where
        T: Transmitter,
        V: Vault,
        R: RngCore + CryptoRng,
{
    fn link_id(&mut self) -> &mut Option<u32> {
        &mut self.link_id
    }

    fn transaction_number(&mut self) -> &mut Option<u8> {
        &mut self.transaction_number
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

impl<T, V, R> MeshContext for Node<T, V, R>
    where
        T: Transmitter,
        V: Vault,
        R: RngCore + CryptoRng,
{
    fn uuid(&self) -> Uuid {
        self.vault.uuid()
    }
}

impl<T, V, R> PipelineContext for Node<T, V, R>
    where
        T: Transmitter,
        V: Vault,
        R: RngCore + CryptoRng,
{
}
