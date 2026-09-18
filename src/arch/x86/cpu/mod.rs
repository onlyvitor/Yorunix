//! Tabelas de CPU de baixo nível: GDT e IDT.
//!
//! Contém apenas estruturas, gates e inicialização — os handlers
//! semânticos vivem em `crate::kernel::interrupts::exceptions`.

pub mod gdt;
pub mod idt;
