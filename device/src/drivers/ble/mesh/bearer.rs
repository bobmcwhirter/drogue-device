pub mod advertising {
    use crate::drivers::ble::mesh::generic_provisioning::{
        GenericProvisioningError, GenericProvisioningPDU,
    };
    use crate::drivers::ble::mesh::PB_ADV;
    use defmt::{write, Format, Formatter};
    use heapless::Vec;

    #[derive(Format)]
    pub struct PDU {
        pub link_id: u32,
        pub transaction_number: u8,
        pub pdu: GenericProvisioningPDU,
    }

    #[derive(Format)]
    pub enum PBAdvError {
        InvalidSize,
        Generic(GenericProvisioningError),
    }

    impl PDU {
        pub fn parse(data: &[u8]) -> Result<PDU, PBAdvError> {
            if data.len() >= 8 {
                defmt::info!("A");
                if data[1] != PB_ADV {
                    defmt::info!("B");
                    Err(PBAdvError::InvalidSize)
                } else {
                    defmt::info!("C: {:x}", data[2..6]);
                    let link_id = u32::from_be_bytes([data[2], data[3], data[4], data[5]]);
                    let transaction_number = data[6];

                    let pdu = GenericProvisioningPDU::parse(&data[7..])
                        .map_err(|e| PBAdvError::Generic(e))?;
                    Ok(PDU {
                        link_id,
                        transaction_number,
                        pdu,
                    })
                }
            } else {
                defmt::info!("D");
                Err(PBAdvError::InvalidSize)
            }
        }

        pub fn emit<const N: usize>(&self, xmit: &mut Vec<u8, N>) {
            xmit.push(0xFF); // placeholder for size.
            xmit.push(PB_ADV);
            xmit.extend_from_slice(&self.link_id.to_be_bytes());
            xmit.push(self.transaction_number);
            self.pdu.emit(xmit);
            xmit[0] = xmit.len() as u8 - 1;
        }
    }
}
