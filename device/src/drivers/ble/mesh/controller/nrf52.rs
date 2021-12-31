use crate::drivers::ble::mesh::device::Uuid;
use crate::drivers::ble::mesh::storage::{Storage, Payload};
use crate::drivers::ble::mesh::transport::{Handler, Transport};
use crate::drivers::ble::mesh::{MESH_BEACON, PB_ADV};
use core::future::Future;
use core::mem;
use core::num::NonZeroU32;
use core::ptr::slice_from_raw_parts;
use embassy::traits::flash::Flash;
use heapless::Vec;
use nrf_softdevice::ble::central::ScanConfig;
use nrf_softdevice::ble::{central, peripheral};
use nrf_softdevice::{random_bytes, raw, Softdevice};
use rand_core::{CryptoRng, Error, RngCore};

pub struct Nrf52BleMeshTransport {
    pub(crate) sd: &'static Softdevice,
}

impl Nrf52BleMeshTransport {
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

    pub fn rng(&self) -> SoftdeviceRng {
        SoftdeviceRng { sd: self.sd }
    }

    pub fn storage(&self, address: usize) -> SoftdeviceStorage {
        SoftdeviceStorage {
            address,
            flash: nrf_softdevice::Flash::take(self.sd),
        }
    }
}

pub struct SoftdeviceRng {
    sd: &'static Softdevice,
}

impl RngCore for SoftdeviceRng {
    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0; 4];
        self.fill_bytes(&mut bytes);
        u32::from_be_bytes(bytes)
    }

    fn next_u64(&mut self) -> u64 {
        let mut bytes = [0; 8];
        self.fill_bytes(&mut bytes);
        u64::from_be_bytes(bytes)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        loop {
            match self.try_fill_bytes(dest) {
                Ok(_) => return,
                Err(_) => {
                    // loop again
                }
            }
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        random_bytes(self.sd, dest).map_err(|_| unsafe { NonZeroU32::new_unchecked(99) }.into())
    }
}

impl CryptoRng for SoftdeviceRng {}

pub struct SoftdeviceStorage {
    address: usize,
    flash: nrf_softdevice::Flash,
}

impl Storage for SoftdeviceStorage {
    type StoreFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), ()>>;

    fn store<'m>(&'m mut self, keys: &'m Payload) -> Self::StoreFuture<'m> {
        async move {
            defmt::info!("store 1 @ {:x}", self.address);
            self.flash.erase(self.address).await.map_err(|_|())?;
            defmt::info!("store 2");
            let result = self.flash.write(self.address, &keys.payload).await;
            defmt::info!("store 3 {}", result);
            Ok(())
        }
    }

    type RetrieveFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<Option<Payload>, ()>>;

    fn retrieve<'m>(&'m mut self) -> Self::RetrieveFuture<'m> {
        async move {
            defmt::info!("retrieve 1");
            let mut payload = [0; 512];
            defmt::info!("retrieve 2");
            self.flash.read(self.address, &mut payload).await.map_err(|_|())?;
            defmt::info!("retrieve 3");
            Ok(Some(Payload { payload }))
        }
    }
}

impl Transport for Nrf52BleMeshTransport {
    fn new() -> Self {
        Self {
            sd: Self::new_sd("Drogue BLE-Mesh"),
        }
    }

    type StartFuture<'m> = impl Future<Output = ()> + 'm;

    fn start<'m>(&'m self) -> Self::StartFuture<'m> {
        async move {
            self.sd.run().await;
        }
    }

    type SendUnprovisionedBeaconFuture<'m> = impl Future<Output = ()> + 'm;

    fn send_unprovisioned_beacon<'m>(
        &'m self,
        uuid: Uuid,
    ) -> Self::SendUnprovisionedBeaconFuture<'m> {
        async move {
            let mut adv_data: Vec<u8, 31> = Vec::new();
            adv_data.extend_from_slice(&[20, MESH_BEACON, 0x00]).ok();

            adv_data.extend_from_slice(&uuid.0).ok();

            adv_data.extend_from_slice(&[0xa0, 0x40]).ok();

            let adv = peripheral::NonconnectableAdvertisement::NonscannableUndirected {
                adv_data: &*adv_data,
            };

            peripheral::advertise(
                self.sd,
                adv,
                &peripheral::Config {
                    max_events: Some(1),
                    ..Default::default()
                },
            )
            .await
            .ok();
        }
    }

    type TransmitFuture<'m> = impl Future<Output = ()> + 'm;

    fn transmit<'m>(&'m self, message: &'m [u8]) -> Self::TransmitFuture<'m> {
        let adv =
            peripheral::NonconnectableAdvertisement::NonscannableUndirected { adv_data: message };

        async move {
            peripheral::advertise(
                self.sd,
                adv,
                &peripheral::Config {
                    max_events: Some(1),
                    //interval: 100,
                    //timeout: Some(300),
                    ..Default::default()
                },
            )
            .await
            .ok();
        }
    }

    type ReceiveFuture<'m, H>
    where
        Self: 'm,
        H: 'm,
    = impl Future<Output = ()> + 'm;

    fn start_receive<'m, H: Handler + 'm>(&'m self, handler: &'m H) -> Self::ReceiveFuture<'m, H> {
        async move {
            //let config = ScanConfig::default();
            let config = ScanConfig {
                interval: 200,
                ..Default::default()
            };
            loop {
                central::scan::<_, ()>(self.sd, &config, |event| {
                    let data = event.data;
                    let data = unsafe { &*slice_from_raw_parts(data.p_data, data.len as usize) };
                    if data.len() >= 2 && data[1] == PB_ADV {
                        handler.handle(Vec::from_slice(data).unwrap());
                    }
                    None
                })
                .await
                .ok();
            }
        }
    }
}
