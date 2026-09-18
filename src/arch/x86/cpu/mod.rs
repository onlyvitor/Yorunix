//! Low-level CPU tables: GDT and IDT.
//!
//! Contains only structures, gates, and initialization — the semantic
//! handlers live in `crate::kernel::interrupts::exceptions`.

pub mod gdt;
pub mod idt;
