use drogue_device::{Actor, Address, Inbox};
use core::future::Future;
use core::ptr::slice_from_raw_parts;
use nrf_softdevice::Softdevice;
use nrf_softdevice::ble::central;
use nrf_softdevice::ble::central::ScanConfig;

pub struct MeshRx {
    sd: &'static Softdevice,
}

impl MeshRx {
    pub fn new(sd: &'static Softdevice) -> Self {
        Self {
            sd
        }
    }
}

impl Actor for MeshRx {
    type OnMountFuture<'m, M>
        where
            Self: 'm,
            M: 'm,
    = impl Future<Output=()> + 'm;

    fn on_mount<'m, M>(&'m mut self, _: Address<Self>, _: &'m mut M) -> Self::OnMountFuture<'m, M> where M: Inbox<Self> + 'm {
        async move {
            let config = ScanConfig::default();
            loop {
                let result = central::scan(self.sd, &config, |event| {
                    let data = event.data;
                    let data = unsafe { &*slice_from_raw_parts(data.p_data, data.len as usize) };
                    //defmt::info!("SCAN {:x}", data);
                    if data.len() > 2 && data[1] == ble_mesh::PB_ADV {
                        defmt::info!("PB-ADV packet received {:x}", data );
                        let packet = ble_mesh::bearer::advertising::PDU::parse(data);
                        defmt::info!("--> {}", packet);
                        Some(packet)
                        //Some(42)
                    } else {
                        None
                    }
                }).await;
            }
        }
    }
}