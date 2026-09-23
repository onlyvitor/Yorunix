//! YoRunix — Rust core (`no_std`, `no_main`, 32-bit x86).
//!
//! This crate is linked as a `staticlib` together with the NASM objects
//! (`entry.o`, `gdt_asm.o`, `idt_asm.o`). `_start` remains in ASM and calls
//! `i686_GDT_Initialize -> idt_init -> kernel_main`.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod arch;
pub mod kernel;

#[cfg(not(test))]
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

const BOOT_BUF_LEN: usize = 160;

/// Composes the boot screen payload: one status line per subsystem plus the
/// welcome banner. `putstr` restarts at (0,0) on every call, so the whole
/// screen must be a single string (same policy as the exception dump).
/// Overflow truncates silently, same as `putstr`.
fn build_boot_text(status: &[(&str, bool)], banner: &str) -> ([u8; BOOT_BUF_LEN], usize) {
    /// Pushes `b`, truncating silently when the buffer is full.
    fn push_byte(buf: &mut [u8; BOOT_BUF_LEN], len: &mut usize, b: u8) {
        if *len < BOOT_BUF_LEN {
            buf[*len] = b;
            *len += 1;
        }
    }

    let mut buf = [0u8; BOOT_BUF_LEN];
    let mut len = 0usize;
    for &(name, ok) in status {
        let tag = if ok { "[ OK ] " } else { "[FAIL] " };
        for &b in tag.as_bytes() {
            push_byte(&mut buf, &mut len, b);
        }
        for &b in name.as_bytes() {
            push_byte(&mut buf, &mut len, b);
        }
        push_byte(&mut buf, &mut len, b'\n');
    }
    for &b in banner.as_bytes() {
        push_byte(&mut buf, &mut len, b);
    }
    (buf, len)
}

/// One status line on the serial log channel (`CR-LF` for TTY terminals).
fn serial_log(name: &str, ok: bool) {
    let tag = if ok { "[ OK ] " } else { "[FAIL] " };
    crate::kernel::drivers::serial::write_str(tag);
    crate::kernel::drivers::serial::write_str(name);
    crate::kernel::drivers::serial::write_str("\r\n");
}

/// Kernel entry point called by `arch/x86/boot/entry.asm`.
/// `->!` documents that it never returns (the `.hang` in ASM is just a fallback).
#[no_mangle]
pub extern "C" fn kernel_main() {
    crate::kernel::drivers::vga::clear_screen();
    crate::kernel::drivers::serial::init();
    crate::kernel::drivers::serial::write_str("Yorunix boot OK (COM1)\r\n");

    let gdt_ok = crate::arch::x86::cpu::gdt::check();
    let idt_ok = crate::arch::x86::cpu::idt::check();
    let vga_ok = crate::kernel::drivers::vga::check();
    let boot_ok = gdt_ok && idt_ok && vga_ok;
    let status: [(&str, bool); 4] = [
        ("gdt", gdt_ok),
        ("idt", idt_ok),
        ("vga", vga_ok),
        ("kernel", boot_ok),
    ];
    for &(name, ok) in &status {
        serial_log(name, ok);
    }

    if !boot_ok {
        crate::kernel::drivers::vga::putstr("Yorunix boot FAILED - see serial log\n");
        crate::kernel::interrupts::exceptions::halt();
    }

    let (buf, len) = build_boot_text(
        &status,
        "Bem-vindo ao Yorunix!\ne magrao isso aqui ta funcionando!",
    );
    // Bytes are pure ASCII by construction, so `from_utf8` cannot fail; the
    // fallback keeps something readable even if composition ever changes.
    let text = core::str::from_utf8(&buf[..len]).unwrap_or("Yorunix\n");
    crate::kernel::drivers::vga::putstr(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_text_lists_status_then_banner() {
        let (buf, len) = build_boot_text(&[("gdt", true), ("idt", false)], "hi\n");
        let text = core::str::from_utf8(&buf[..len]).unwrap();
        assert_eq!(text, "[ OK ] gdt\n[FAIL] idt\nhi");
    }

    #[test]
    fn boot_text_truncates_when_full() {
        let banner = "x".repeat(BOOT_BUF_LEN);
        let (buf, len) = build_boot_text(&[("gdt", true)], &banner);
        assert_eq!(len, BOOT_BUF_LEN);
        // The status line survives; the banner was truncated at the end.
        assert!(buf.starts_with(b"[ OK ] gdt\n"));
    }
}
