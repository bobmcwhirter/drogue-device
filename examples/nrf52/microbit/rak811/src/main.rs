#![no_std]
#![no_main]
#![macro_use]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(concat_idents)]

mod app;

use app::*;

use log::LevelFilter;
use panic_probe as _;
use rtt_logger::RTTLogger;
use rtt_target::rtt_init_print;

use core::cell::UnsafeCell;
use drogue_device::{
    actors::button::Button, drivers::lora::rak811::*, traits::lora::*, ActorContext, DeviceContext,
};
use embassy::util::Forever;
use embassy_nrf::{
    buffered_uarte::{BufferedUarte, State},
    gpio::{Input, Level, NoPin, Output, OutputDrive, Pull},
    gpiote::PortInput,
    interrupt,
    peripherals::{P0_14, P1_02, TIMER0, UARTE0},
    uarte, Peripherals,
};

const DEV_EUI: &str = include_str!(concat!(env!("OUT_DIR"), "/config/dev_eui.txt"));
const APP_EUI: &str = include_str!(concat!(env!("OUT_DIR"), "/config/app_eui.txt"));
const APP_KEY: &str = include_str!(concat!(env!("OUT_DIR"), "/config/app_key.txt"));

static LOGGER: RTTLogger = RTTLogger::new(LevelFilter::Info);

type UART = BufferedUarte<'static, UARTE0, TIMER0>;
type RESET = Output<'static, P1_02>;

pub struct MyDevice {
    driver: UnsafeCell<Rak811Driver>,
    modem: ActorContext<'static, Rak811ModemActor<'static, UART, RESET>>,
    app: ActorContext<'static, App<Rak811Controller<'static>>>,
    button: ActorContext<
        'static,
        Button<'static, PortInput<'static, P0_14>, App<Rak811Controller<'static>>>,
    >,
}

static DEVICE: DeviceContext<MyDevice> = DeviceContext::new();

#[embassy::main]
async fn main(spawner: embassy::executor::Spawner, p: Peripherals) {
    rtt_init_print!();
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    let button_port = PortInput::new(Input::new(p.P0_14, Pull::Up));

    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::EXCLUDED;
    config.baudrate = uarte::Baudrate::BAUD115200;

    static mut TX_BUFFER: [u8; 256] = [0u8; 256];
    static mut RX_BUFFER: [u8; 256] = [0u8; 256];
    static mut STATE: Forever<State<'static, UARTE0, TIMER0>> = Forever::new();

    let irq = interrupt::take!(UARTE0_UART0);
    let u = unsafe {
        let state = STATE.put(State::new());
        BufferedUarte::new(
            state,
            p.UARTE0,
            p.TIMER0,
            p.PPI_CH0,
            p.PPI_CH1,
            irq,
            p.P0_13,
            p.P0_01,
            NoPin,
            NoPin,
            config,
            &mut RX_BUFFER,
            &mut TX_BUFFER,
        )
    };

    let reset_pin = Output::new(p.P1_02, Level::High, OutputDrive::Standard);

    let config = LoraConfig::new()
        .region(LoraRegion::EU868)
        .lora_mode(LoraMode::WAN)
        .device_eui(&DEV_EUI.trim_end().into())
        .app_eui(&APP_EUI.trim_end().into())
        .app_key(&APP_KEY.trim_end().into());

    DEVICE.configure(MyDevice {
        driver: UnsafeCell::new(Rak811Driver::new()),
        modem: ActorContext::new(Rak811ModemActor::new()),
        app: ActorContext::new(App::new(config)),
        button: ActorContext::new(Button::new(button_port)),
    });

    DEVICE
        .mount(|device| async move {
            let (controller, modem) = unsafe { &mut *device.driver.get() }.initialize(u, reset_pin);
            device.modem.mount(modem, spawner);
            let app = device.app.mount(controller, spawner);
            device.button.mount(app, spawner);
        })
        .await;
}
