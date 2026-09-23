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

fn init_serial(port: u16) {
    outb(port + 1, 0x00);
    outb(port + 3, 0x80);
    outb(port, 0x03);
    outb(port + 1, 0x00);
    outb(port + 3, 0x03);
    outb(port + 2, 0xC7);
    outb(port + 4, 0x0B);
    outb(port + 4, 0x1E);
    outb(port, 0xAE);

    if inb(port) != 0xAE {
        return;
    }
    outb(port + 4, 0x0f);
}

pub fn init_com1() {
    init_serial(COM1);
}
