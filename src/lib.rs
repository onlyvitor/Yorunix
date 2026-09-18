//! YoRunix — Rust core (`no_std`, `no_main`, 32-bit x86).
//!
//! This crate is linked as a `staticlib` together with the NASM objects
//! (`entry.o`, `gdt_asm.o`, `idt_asm.o`). `_start` remains in ASM and calls
//! `i686_GDT_Initialize -> idt_init -> kernel_main`.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod arch;
pub mod kernel;

use core::arch::asm;
#[cfg(not(test))]
use core::panic::PanicInfo;

/// No runtime: a panic just safely halts the CPU.
///
/// Compiled only outside `cargo test`: on the test host `std` already provides
/// the handler, and two `#[panic_handler]`s would be `duplicate lang item panic_impl`.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // SAFETY: `hlt` with interrupts potentially enabled is safe;
        // if an NMI wakes up, the loop halts again. No memory access.
        unsafe { asm!("hlt", options(nomem, nostack, preserves_flags)) };
    }
}

/// Kernel entry point called by `arch/x86/boot/entry.asm`.
/// `->!` documents that it never returns (the `.hang` in ASM is just a fallback).
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    unsafe {
        core::arch::asm!("mov eax, 1; xor edx, edx; mov ecx, 0; div ecx", out("eax") _, out("edx") _, out("ecx") _, options(nostack));
    }
    loop {
        // SAFETY: `hlt` saves power until the next interrupt; the loop
        // guarantees we never return to the ASM caller.
        unsafe { asm!("hlt", options(nomem, nostack, preserves_flags)) };
    }
}
