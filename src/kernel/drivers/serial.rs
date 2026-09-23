use core::arch::asm;

const COM1: u16 = 0x3F8;

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

pub fn init() {
    outb(COM1 + 1, 0x00);
    outb(COM1 + 3, 0x80);
    outb(COM1, 0x03);
    outb(COM1 + 1, 0x00);
    outb(COM1 + 3, 0x03);
    outb(COM1 + 2, 0xC7);
    outb(COM1 + 4, 0x0B);
    outb(COM1 + 4, 0x1E);
    outb(COM1, 0xAE);

    if inb(COM1) != 0xAE {
        return;
    }
    outb(COM1 + 4, 0x0f);
}

/// Writes a single byte to COM1, busy-waiting until the transmitter
/// holding register is empty (Line Status Register, port + 5, bit 5).
pub fn write_byte(byte: u8) {
    while inb(COM1 + 5) & 0x20 == 0 {}
    outb(COM1, byte);
}

/// Writes a string to COM1.
pub fn write_str(text: &str) {
    for &byte in text.as_bytes() {
        write_byte(byte);
    }
}
