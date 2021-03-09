#[cfg(any(feature = "stm32l4xx", feature = "chip+stm32l4xx"))]
pub mod stm32l4xx;

#[cfg(any(feature = "stm32l1xx"))]
pub mod stm32l1xx;

#[cfg(any(feature = "chip+nrf51", feature = "chip+nrf52833",))]
pub mod nrf;