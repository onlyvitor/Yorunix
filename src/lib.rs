//! YoRunix — núcleo Rust (`no_std`, `no_main`, 32-bit x86).
//!
//! Este crate é linkado como `staticlib` junto aos objetos NASM
//! (`entry.o`, `gdt_asm.o`, `idt_asm.o`). O `_start` continua em ASM e chama
//! `i686_GDT_Initialize -> idt_init -> kernel_main`.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod gdt;
pub mod handler;
pub mod idt;
#[cfg(not(test))]
pub mod support;
pub mod vga;

use core::arch::asm;
#[cfg(not(test))]
use core::panic::PanicInfo;

/// Sem runtime: pânico apenas trava a CPU de forma segura.
///
/// Compilado só fora de `cargo test`: no host de teste o `std` já fornece o
/// handler, e dois `#[panic_handler]` seriam `duplicate lang item panic_impl`.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // SAFETY: `hlt` com interrupções potencialmente habilitadas é seguro;
        // se uma NMI acordar, o loop volta a haltar. Sem acesso a memória.
        unsafe { asm!("hlt", options(nomem, nostack, preserves_flags)) };
    }
}

/// Ponto de entrada do kernel chamado pelo `boot/entry.asm`.
/// `->!` documenta que nunca retorna (o `.hang` no ASM é só fallback).
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    vga::clear_screen();
    vga::putstr("Bem-vindo ao Yorunix!");
    loop {
        // SAFETY: `hlt` economiza energia até a próxima interrupção; o loop
        // garante que nunca retornamos ao chamador ASM.
        unsafe { asm!("hlt", options(nomem, nostack, preserves_flags)) };
    }
}
