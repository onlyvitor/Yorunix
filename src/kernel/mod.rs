//! Architecture-independent kernel logic.
//!
//! - `interrupts`: conceptual handling of exceptions/IRQs.
//! - `drivers`: hardware/drivers (VGA today, serial in the future).
//! - `support`: freestanding symbols required by the linker.

pub mod drivers;
pub mod interrupts;
#[cfg(not(test))]
pub mod support;
