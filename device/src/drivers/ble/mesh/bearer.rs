pub mod advertising {
    use crate::drivers::ble::mesh::generic_provisioning::GenericProvisioningPDU;
    use crate::drivers::ble::mesh::PB_ADV;
    use defmt::{write, Format, Formatter};
    use heapless::Vec;

    pub struct PDU {
        link_id: u32,
        transaction_number: u8,
        pdu: GenericProvisioningPDU,
    }

    impl Format for PDU {
        fn format(&self, fmt: Formatter) {
            write!(
                fmt,
                "link_id: {}; transaction_number: {}; pdu: {}",
                self.link_id, self.transaction_number, self.pdu
            );
        }
    }

    impl PDU {
        pub fn parse(data: &[u8]) -> Result<PDU, ()> {
            if data.len() >= 8 {
                if data[1] != PB_ADV {
                    Err(())
                } else {
                    let link_id = u32::from_be_bytes([data[2], data[3], data[4], data[5]]);
                    let transaction_number = data[6];

                    let pdu = GenericProvisioningPDU::parse(&data[7..])?;
                    Ok(PDU {
                        link_id,
                        transaction_number,
                        pdu,
                    })
                }
            } else {
                Err(())
            }
        }
    }
}
