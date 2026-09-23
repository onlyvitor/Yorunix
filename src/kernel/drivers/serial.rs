//! COM1 serial output over a 16550 UART, driven by polling.
//!
//! Call `init()` before any write: it programs the port and verifies
//! it is alive with a loopback test.

use core::arch::asm;

/// Base I/O address of COM1 on x86 PCs. Its registers live at offsets
/// 0..=7 from this port.
const COM1: u16 = 0x3F8;

/// Writes a byte to an I/O port.
fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            // The asm touches only I/O ports, not Rust memory.
            options(nomem, nostack, preserves_flags)
        )
    }
}

/// Reads a byte from an I/O port.
///
/// A port with nothing attached usually answers 0xFF.
fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Programs COM1 for polled output (38400 baud, 8N1) and verifies it
/// with a loopback test.
pub fn init() {
    // IER (port + 1): disable all UART interrupts; we poll instead.
    outb(COM1 + 1, 0x00);
    // LCR (port + 3): set DLAB so ports 0/1 become the baud divisor.
    outb(COM1 + 3, 0x80);
    // Divisor = 3 -> 115200 / 3 = 38400 baud.
    outb(COM1, 0x03); // DLL: divisor low byte
    outb(COM1 + 1, 0x00); // DLM: divisor high byte
                          // LCR: DLAB off, 8 data bits, no parity, 1 stop bit.
    outb(COM1 + 3, 0x03);
    // FCR (port + 2): enable and clear FIFOs, 14-byte trigger.
    outb(COM1 + 2, 0xC7);
    // MCR (port + 4): raise DTR, RTS and OUT2.
    outb(COM1 + 4, 0x0B);
    // MCR: loopback mode -- written bytes echo straight back.
    outb(COM1 + 4, 0x1E);
    outb(COM1, 0xAE); // test byte

    // A healthy port echoes 0xAE; a dead one answers anything else.
    if inb(COM1) != 0xAE {
        // Give up silently for now; the caller is not told yet.
        return;
    }
    // MCR: leave loopback, keep modem lines and OUT2 raised.
    outb(COM1 + 4, 0x0f);
}

/// Writes a single byte to COM1, busy-waiting until the transmitter
/// holding register is empty (Line Status Register, port + 5, bit 5).
pub fn write_byte(byte: u8) {
    while inb(COM1 + 5) & 0x20 == 0 {}
    outb(COM1, byte);
}

/// Writes a string to COM1, one byte at a time, blocking per byte.
pub fn write_str(text: &str) {
    for &byte in text.as_bytes() {
        write_byte(byte);
    }
}
