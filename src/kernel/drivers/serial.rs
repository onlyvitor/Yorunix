#![allow(dead_code)]

use core::arch::asm;

const COM1: u16 = 0x3F8;

struct SerialPort {
    port: u16,
}

fn outb(serial: SerialPort, value: u8) {
    let port = serial.port;

    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        )
    }
}

pub fn init_com1() {}
