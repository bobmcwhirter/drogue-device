#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

mod controller;
mod ble_mesh;

#[cfg(feature = "defmt-rtt")]
use defmt_rtt as _;

use drogue_device::{ActorContext, Address, Board, DeviceContext, Package, actors, drivers, Actor};
use embassy::executor::Spawner;
use embassy_nrf::config::Config;
use embassy_nrf::interrupt::Priority;
use embassy_nrf::{
    gpio::{AnyPin, Output},
    Peripherals,
};
use embassy_nrf::gpio::{Level, OutputDrive, Pin};
use embassy_nrf::peripherals::P0_13;
use nrf_softdevice::Softdevice;

use panic_probe as _;

use crate::controller::BleController;
use heapless::Vec;
use crate::ble_mesh::MeshBleService;


pub struct MyDevice {
    led: ActorContext<actors::led::Led<drivers::led::Led<Output<'static, AnyPin>>>>,
    mesh: MeshBleService,
}

static DEVICE: DeviceContext<MyDevice> = DeviceContext::new();

// Application must run at a lower priority than softdevice
fn config() -> Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    config
}


#[embassy::main(config = "config()")]
async fn main(spawner: Spawner, p: Peripherals) {
    let device = DEVICE.configure(MyDevice {
        led: ActorContext::new(),
        mesh: MeshBleService::new(),
    });

    device.mesh.mount( (), spawner);

    defmt::info!("Started");
}