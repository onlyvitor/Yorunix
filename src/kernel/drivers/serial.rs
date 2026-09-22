#![allow(dead_code)]

use core::arch::asm;

const COM1: u16 = 0x3F8;

struct SerialPort {
    port: u16,
}

fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        )
    }
}

/// Reads a byte from the given I/O port.
///
/// # Safety
///
/// Caller must ensure `port` is a valid readable I/O port.
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nomem, nostack, preserves_flags)
    );
    value
}

fn init_serial(serial: SerialPort) {
    outb(serial.port + 1, 0x00);
    outb(serial.port + 3, 0x80);
    outb(serial.port, 0x03);
    outb(serial.port + 1, 0x00);
    outb(serial.port + 3, 0x03);
    outb(serial.port + 2, 0xC7);
    outb(serial.port + 4, 0x0B);
    outb(serial.port + 4, 0x1E);
    outb(serial.port, 0xAE);
}

pub fn init_com1() {}
