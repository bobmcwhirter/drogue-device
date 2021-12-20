#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

#[cfg(feature = "defmt-rtt")]
use defmt_rtt as _;

use drogue_device::actors::ble::mesh::BleMesh;
use drogue_device::drivers::ble::mesh::controller::nrf52::Nrf52BleMeshTransport;
use drogue_device::drivers::ble::mesh::device::Uuid;
use drogue_device::drivers::ble::mesh::transport::Transport;
use drogue_device::{actors, drivers, Actor, ActorContext, Address, Board, DeviceContext, Package};
use embassy::executor::Spawner;
use embassy_nrf::config::Config;
use embassy_nrf::gpio::{Level, OutputDrive, Pin};
use embassy_nrf::interrupt::Priority;
use embassy_nrf::peripherals::P0_13;
use embassy_nrf::{
    gpio::{AnyPin, Output},
    Peripherals,
};
use nrf_softdevice::Softdevice;

use panic_probe as _;

use heapless::Vec;

pub struct MyDevice {
    led: ActorContext<actors::led::Led<drivers::led::Led<Output<'static, AnyPin>>>>,
    mesh: BleMesh<Nrf52BleMeshTransport>,
}

static DEVICE: DeviceContext<MyDevice> = DeviceContext::new();

// Application must run at a lower priority than softdevice
fn config() -> Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    config
}

const NODE_UUID: Uuid = Uuid([
    0xBE, 0xEF, 0xBE, 0xEF, 0xBE, 0xEF, 0xBE, 0xEF, 0xBE, 0xEF, 0xBE, 0xEF, 0xBE, 0xEF, 0xBE, 0xEF,
]);

#[embassy::main(config = "config()")]
async fn main(spawner: Spawner, p: Peripherals) {
    let device = DEVICE.configure(MyDevice {
        led: ActorContext::new(),
        mesh: BleMesh::new(Nrf52BleMeshTransport::new()),
    });

    device.mesh.mount(NODE_UUID, spawner);

    defmt::info!("Started");
}
