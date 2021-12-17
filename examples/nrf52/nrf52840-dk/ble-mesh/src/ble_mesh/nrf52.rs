use core::cell::UnsafeCell;
use core::future::Future;
use core::mem;
use core::ptr::slice_from_raw_parts;
use ble_mesh::transport::{Receiver, Transmitter, Transport};
use drogue_device::{Actor, ActorContext, ActorSpawner, Address, Inbox, Package};
use embassy::executor::Spawner;
use nrf_softdevice::{raw, Softdevice};
use nrf_softdevice::ble::central::ScanConfig;
use nrf_softdevice::ble::{central, peripheral};

pub struct Nrf52BleMeshController {
    receiver: UnsafeCell<Option<Receiver>>,
    sd: &'static Softdevice,
    sd_actor: ActorContext<SoftdeviceActor>,
    rx: ActorContext<Rx>,
    tx: ActorContext<Tx>,
}

impl Nrf52BleMeshController {
    fn new_sd(device_name: &'static str) -> &'static Softdevice {
        let config = nrf_softdevice::Config {
            clock: Some(raw::nrf_clock_lf_cfg_t {
                source: raw::NRF_CLOCK_LF_SRC_RC as u8,
                rc_ctiv: 4,
                rc_temp_ctiv: 2,
                accuracy: 7,
            }),
            conn_gap: Some(raw::ble_gap_conn_cfg_t {
                conn_count: 6,
                event_length: 24,
            }),
            conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
            gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
                attr_tab_size: 32768,
            }),
            gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
                adv_set_count: 1,
                periph_role_count: 3,
                central_role_count: 1,
                central_sec_count: 1,
                _bitfield_1: Default::default(),
            }),
            gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
                p_value: device_name.as_ptr() as *const u8 as _,
                current_len: device_name.len() as u16,
                max_len: device_name.len() as u16,
                write_perm: unsafe { mem::zeroed() },
                _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                    raw::BLE_GATTS_VLOC_STACK as u8,
                ),
            }),

            ..Default::default()
        };
        let sd = Softdevice::enable(&config);
        sd
    }
}

impl Transport for Nrf52BleMeshController {
    fn new(receiver: Receiver) -> Self {
        Self {
            sd: Self::new_sd("BLE Mesh"),
            sd_actor: ActorContext::new(),
            rx: ActorContext::new(),
            tx: ActorContext::new(),
            receiver: UnsafeCell::new(Some(receiver)),
        }
    }

    fn start(&'static self, spawner: Spawner) {
        self.mount((), spawner);
    }
}

impl Transmitter for Address<Tx> {

    type TransmitFuture<'m> = impl Future<Output=()> + 'm;

    fn transmit<'m>(&self, message: u8) -> Self::TransmitFuture<'m> {
        async move {}
    }
}

impl Package for Nrf52BleMeshController
{
    type Primary = Tx;
    type Configuration = ();

    fn mount<S: ActorSpawner>(
        &'static self,
        _config: Self::Configuration,
        spawner: S,
    ) -> Address<Self::Primary> {
        let _ = self.sd_actor.mount(spawner, SoftdeviceActor(self.sd));

        self.rx.mount(
            spawner,
            Rx { sd: self.sd, receiver: unsafe { (&mut *self.receiver.get()).take().unwrap() } },
        );

        self.tx.mount(
            spawner,
            Tx{ sd: self.sd},
        )
    }
}

struct SoftdeviceActor(&'static Softdevice);

impl Actor for SoftdeviceActor {
    type OnMountFuture<'m, M>
        where
            Self: 'm,
            M: 'm,
    = impl Future<Output=()> + 'm;

    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M> where M: Inbox<Self> + 'm {
        async move {
            self.0.run().await;
        }
    }
}

struct Rx {
    sd: &'static Softdevice,
    receiver: Receiver,
}

impl Actor for Rx {
    type OnMountFuture<'m, M>
        where
            Self: 'm,
            M: 'm,
    = impl Future<Output=()> + 'm;
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M>
        where
            M: Inbox<Self> + 'm,
    {
        async move {
            let config = ScanConfig::default();
            loop {
                let result = central::scan(self.sd, &config, |event| {
                    let data = event.data;
                    let data = unsafe { &*slice_from_raw_parts(data.p_data, data.len as usize) };
                    //defmt::info!("SCAN {:x}", data);
                    if data.len() >= 2 && data[1] == ble_mesh::PB_ADV {
                        self.receiver.receive(data);
                        //defmt::info!("PB-ADV packet received {:x}", data );
                        //let packet = ble_mesh::bearer::advertising::PDU::parse(data);
                        //defmt::info!("--> {}", packet);
                        //Some(packet)
                        Some(42)
                        //None
                    } else {
                        None
                    }
                }).await;
            }
        }
    }
}

pub struct Tx {
    sd: &'static Softdevice,
}

pub struct Transmit<'m>(pub &'m [u8]);

impl Actor for Tx {
    type Message<'m> where Self: 'm = Transmit<'m>;
    type OnMountFuture<'m, M>
        where
            Self: 'm,
            M: 'm,
    = impl Future<Output=()> + 'm;
    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, inbox: &'m mut M) -> Self::OnMountFuture<'m, M>
        where
            M: Inbox<Self> + 'm,
    {
        async move {
            loop {
                defmt::info!("loop");
                match inbox.next().await {
                    Some(ref mut message) => {
                        let xmit = message.message();
                        let adv = peripheral::NonconnectableAdvertisement::NonscannableUndirected {
                            adv_data: xmit.0,
                        };

                        peripheral::advertise(self.sd, adv, &peripheral::Config {
                            max_events: Some(1),
                            ..Default::default()
                        }).await;
                    }
                    None => {

                    }
                }
            }
        }
    }
}