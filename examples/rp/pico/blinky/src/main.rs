#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use actors::led::LedMessage;
use defmt_rtt as _;
use drogue_device::{actors, drivers, ActorContext, DeviceContext};
use embassy_rp::{
    gpio::{Level, Output},
    peripherals::PIN_25,
    Peripherals,
};

use panic_probe as _;

pub struct MyDevice {
    //led: ActorContext<'static, Led<Output<'static, PIN_25>>>,
    led: ActorContext<'static, actors::led::Led<drivers::led::Led<Output<'static, PIN_25>>>>,
}

static DEVICE: DeviceContext<MyDevice> = DeviceContext::new();

#[embassy::main]
async fn main(spawner: embassy::executor::Spawner, p: Peripherals) {
    DEVICE.configure(MyDevice {
        led: ActorContext::new(actors::led::Led::new(drivers::led::Led::new(Output::new(
            p.PIN_25,
            Level::Low,
        )))),
    });

    let led = DEVICE
        .mount(|device| async move { device.led.mount((), spawner) })
        .await;

    loop {
        cortex_m::asm::delay(1_000_000);
        led.request(LedMessage::Toggle).unwrap().await;
    }
}
