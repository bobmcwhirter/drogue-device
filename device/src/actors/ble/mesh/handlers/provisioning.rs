use crate::actors::ble::mesh::device::Device;
use crate::drivers::ble::mesh::provisioning::{ProvisioningPDU, PublicKey};
use crate::drivers::ble::mesh::transport::Transport;
use core::convert::TryFrom;
use core::marker::PhantomData;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::elliptic_curve::AffineXCoordinate;

enum State {
    None,
    Invite,
    Capabilities,
    Start,
    PublicKey,
    InputComplete,
    Confirmation,
    Random,
    Data,
    Complete,
    Failed,
}

pub struct ProvisioningHandler<T: Transport + 'static> {
    state: State,
    _marker: PhantomData<T>,
}

impl<T: Transport + 'static> ProvisioningHandler<T> {
    pub(crate) fn new() -> Self {
        Self {
            state: State::None,
            _marker: PhantomData,
        }
    }

    pub(crate) async fn handle(
        &mut self,
        device: &Device<T>,
        pdu: ProvisioningPDU,
    ) -> Result<(), ()> {
        match pdu {
            ProvisioningPDU::Invite(invite) => {
                defmt::info!(">> ProvisioningPDU::Invite");
                self.state = State::Invite;
                device.tx_capabilities()?;
            }
            ProvisioningPDU::Capabilities(_) => {}
            ProvisioningPDU::Start(start) => {
                defmt::info!(">> ProvisioningPDU::Start");
            }
            ProvisioningPDU::PublicKey(public_key) => {
                defmt::info!(">> ProvisioningPDU::PublicKey");
                let pk = device.public_key();
                let xy = pk.to_encoded_point(false);
                let x = xy.x().unwrap();
                let y = xy.y().unwrap();
                device.tx_provisioning_pdu(ProvisioningPDU::PublicKey(PublicKey {
                    x: <[u8; 32]>::try_from(x.as_slice()).map_err(|_| ())?,
                    y: <[u8; 32]>::try_from(y.as_slice()).map_err(|_| ())?,
                }));
                //device.tx_provisioning_pdu(ProvisioningPDU::InputComplete);
            }
            ProvisioningPDU::InputComplete => {
                defmt::info!(">> ProvisioningPDU::InputComplete");
            }
            ProvisioningPDU::Confirmation(confirmation) => {
                defmt::info!(">> ProvisioningPDU::Confirmation {}", confirmation);
            }
            ProvisioningPDU::Random(random) => {
                defmt::info!(">> ProvisioningPDU::Random");
            }
            ProvisioningPDU::Data(data) => {
                defmt::info!(">> ProvisioningPDU::Data");
            }
            ProvisioningPDU::Complete => {
                defmt::info!(">> ProvisioningPDU::Complete");
            }
            ProvisioningPDU::Failed(failed) => {
                defmt::info!(">> ProvisioningPDU::Failed");
            }
        }
        Ok(())
    }
}
