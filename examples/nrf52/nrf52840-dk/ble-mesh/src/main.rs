#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

#[cfg(feature = "defmt-rtt")]
use defmt_rtt as _;
use drogue_device::{ActorContext, actors, DeviceContext, drivers, Package};
use drogue_device::actors::ble::mesh::BleMesh;
use drogue_device::drivers::ble::mesh::controller::nrf52::{Nrf52BleMeshTransport, SoftdeviceKeyStorage, SoftdeviceRng};
use drogue_device::drivers::ble::mesh::device::Uuid;
use drogue_device::drivers::ble::mesh::provisioning::{
    Algorithms, Capabilities, InputOOBActions, OOBSize, OutputOOBActions, PublicKeyType,
    StaticOOBType,
};
use drogue_device::drivers::ble::mesh::transport::Transport;
use embassy::executor::Spawner;
use embassy_nrf::{
    gpio::{AnyPin, Output},
    Peripherals,
};
use embassy_nrf::config::Config;
//use embassy_nrf::interrupt;
use embassy_nrf::interrupt::Priority;
//use nrf_softdevice::Softdevice as _;
use panic_probe as _;

pub struct MyDevice {
    led: ActorContext<actors::led::Led<drivers::led::Led<Output<'static, AnyPin>>>>,
    mesh: BleMesh<Nrf52BleMeshTransport, SoftdeviceRng, SoftdeviceKeyStorage>,
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

extern "C" {
    static __key_storage: u8;
}

#[embassy::main(config = "config()")]
async fn main(spawner: Spawner, p: Peripherals) {
    let transport = Nrf52BleMeshTransport::new();
    let rng = transport.rng();
    let storage = transport.key_storage(unsafe { &__key_storage as *const u8 as usize } );

    let device = DEVICE.configure(MyDevice {
        led: ActorContext::new(),
        mesh: BleMesh::new(transport)
    });

    let capabilities = Capabilities {
        number_of_elements: 1,
        algorithms: Algorithms::default(),
        public_key_type: PublicKeyType::default(),
        static_oob_type: StaticOOBType::default(),
        output_oob_size: OOBSize::MaximumSize(4),
        output_oob_action: OutputOOBActions::default(),
        input_oob_size: OOBSize::MaximumSize(4),
        input_oob_action: InputOOBActions::default(),
    };

    device.mesh.mount((rng, storage, NODE_UUID, capabilities), spawner);
}
