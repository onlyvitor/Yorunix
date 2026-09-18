//! Lógica do kernel independente de arquitetura.
//!
//! - `interrupts`: tratamento conceitual de exceções/IRQs.
//! - `drivers`: hardware/drivers (VGA hoje, serial no futuro).
//! - `support`: símbolos freestanding exigidos pelo linker.

pub mod drivers;
pub mod interrupts;
#[cfg(not(test))]
pub mod support;
