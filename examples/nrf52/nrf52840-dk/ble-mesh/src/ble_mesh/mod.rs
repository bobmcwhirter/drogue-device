use drogue_device::{ActorContext, ActorSpawner, Address, Package};
use heapless::Vec;
use nrf_softdevice::Softdevice;
use crate::ble_mesh::tx::MeshTx;
use crate::ble_mesh::rx::MeshRx;
use crate::BleController;

mod tx;
mod rx;

pub struct MeshBleService
{
    sd: &'static Softdevice,
    controller: ActorContext<BleController>,
    tx: ActorContext<MeshTx>,
    rx: ActorContext<MeshRx>,
}

impl MeshBleService
{
    pub fn new() -> Self {
        let sd = BleController::new_sd("Drogue IoT Mesh Device");
        Self {
            sd,
            controller: ActorContext::new(),
            tx: ActorContext::new(),
            rx: ActorContext::new(),
        }
    }
}

impl Package for MeshBleService
{
    type Primary = BleController;
    type Configuration = ();

    fn mount<S: ActorSpawner>(
        &'static self,
        _config: Self::Configuration,
        spawner: S,
    ) -> Address<Self::Primary> {
        let controller = self.controller.mount(spawner, BleController::new(self.sd));

        self.rx.mount(
            spawner,
            MeshRx::new(self.sd)
        );

        self.tx.mount(
            spawner,
            MeshTx::new(self.sd)
        );
        controller
    }
}
