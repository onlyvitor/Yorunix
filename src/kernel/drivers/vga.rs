//! VGA text-mode driver — safe wrapper over MMIO 0xB8000.
//!
//! Fixes bugs in the original `drivers/vga.c`:
//! - `clear_screen` wrote `0x00` (NUL) and iterated `i < 80*25` with step 2
//!   (clearing only half). It now writes `b' '` to all 2000 cells.
//! - Writes now use `write_volatile` so the compiler does not elide MMIO.
//! - The original `putstr` handled neither `\n`, wrapping, nor screen limits.

use core::ptr::{read_volatile, write_volatile};

pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;
pub const VGA_ADDR: usize = 0xB8000;

/// Bright white on black, same `WHITE_COLOR 0x0F` as in C.
pub const COLOR_WHITE_ON_BLACK: u8 = 0x0F;

#[repr(C)]
#[derive(Clone, Copy)]
struct ScreenCell {
    ascii: u8,
    color: u8,
}

const BLANK_CELL: ScreenCell = ScreenCell {
    ascii: b' ',
    color: COLOR_WHITE_ON_BLACK,
};

fn buffer() -> *mut ScreenCell {
    VGA_ADDR as *mut ScreenCell
}

/// Converts a `u32` into 8 ASCII hexadecimal digits (uppercase, MSB first).
///
/// Pure function: it does not touch MMIO, hence it is testable on the host (`cargo test`).
/// Always returns zero-padded output (e.g. `0x00F0000A` -> `b"00F0000A"`), so
/// `eip/cs/eflags` dumps have fixed width and are easy to compare.
/// No `core::fmt`/alloc — important with `panic = "abort"` and `no_std`.
fn hex_digits(v: u32) -> [u8; 8] {
    // Nibble (4 bits) -> ASCII conversion table. `b'A'` base guarantees
    // uppercase, the kernel dump standard (`0x1234ABCD`, not `0x1234abcd`).
    let mut out = [b'0'; 8];
    for (i, slot) in out.iter_mut().enumerate() {
        // Extracts nibble `i` from most to least significant:
        // i=0 -> bits 28..31, i=7 -> bits 0..3.
        let shift = 28 - (i as u32) * 4;
        let nibble = ((v >> shift) & 0xF) as u8;
        // 0..9 -> '0'..'9', 10..15 -> 'A'..'F'.
        *slot = if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + (nibble - 10)
        };
    }
    out
}

/// Clears the whole screen to blank spaces.
pub fn clear_screen() {
    // SAFETY: 0xB8000 is the VGA text-mode buffer mapped by the hardware/BIOS on
    // x86. The kernel is the sole owner after boot; a volatile write of 2000
    // in-bounds cells is always valid. Called with interrupts
    // already disabled in `_start` or in a single-core boot context.
    unsafe {
        let buf = buffer();
        let total = VGA_WIDTH * VGA_HEIGHT;
        for i in 0..total {
            write_volatile(buf.add(i), BLANK_CELL);
        }
    }
}

/// Writes `s` from the top-left, with `\n`, wrapping, and safe truncation.
///
/// Keeps a simple internal cursor (no scrolling for now — same visible
/// semantics as the original `putstr` for parity under QEMU).
pub fn putstr(s: &str) {
    // SAFETY: same owner/address as `clear_screen`. A volatile read of the
    // cursor is unnecessary (local state); each write is volatile and
    // bound-checked with `if offset >= total { break }`.
    unsafe {
        let buf = buffer();
        let total = VGA_WIDTH * VGA_HEIGHT;
        let mut offset: usize = 0;
        for &b in s.as_bytes() {
            if b == b'\n' {
                // Advance to the start of the next line.
                let row = offset / VGA_WIDTH;
                offset = (row + 1) * VGA_WIDTH;
                if offset >= total {
                    break;
                }
                continue;
            }
            if offset >= total {
                break;
            }
            // Preserve the cell's current color (allows future highlighting),
            // but guarantee a printable character.
            let color = read_volatile(buf.add(offset)).color;
            let color = if color == 0 {
                COLOR_WHITE_ON_BLACK
            } else {
                color
            };
            write_volatile(
                buf.add(offset),
                ScreenCell {
                    ascii: if b == 0 { b' ' } else { b },
                    color,
                },
            );
            offset += 1;
        }
    }
}

/// Writes `v` as `0xXXXXXXXX` (10 cells) at position `(row, col)`.
///
/// Positional model (2B): no global cursor, no side effect on `putstr`.
/// Ideal for exception dumps (`debug()` prints `eip` on row 1 without erasing
/// the row 0 header). Does not wrap to the next row: if `col + 10`
/// exceeds `VGA_WIDTH`, it truncates — preventing a dump from corrupting the
/// screen layout. An off-screen position (`row >= 25` or `col >= 80`) is a safe no-op.
pub fn put_hex_at(v: u32, row: usize, col: usize) {
    // SAFETY: same owner/address as `clear_screen`/`putstr` (MMIO 0xB8000,
    // sole owner is the kernel). Each write is `volatile` and preceded by a
    // bound check, so we never write outside the 2000 cells. Reading the
    // current color via `read_volatile` follows the `putstr` pattern.
    unsafe {
        // Reject an invalid origin before touching the hardware.
        if row >= VGA_HEIGHT || col >= VGA_WIDTH {
            return;
        }
        // Build the `0x` + 8-digit text in a local buffer (no alloc, no fmt).
        let digits = hex_digits(v);
        let mut text = [b'0'; 10];
        text[0] = b'0';
        text[1] = b'x';
        text[2..].copy_from_slice(&digits);

        let buf = buffer();
        let base = row * VGA_WIDTH + col;
        // Remaining width on this row: guarantees truncation without wrapping.
        let room = VGA_WIDTH - col;
        let len = if text.len() < room { text.len() } else { room };
        for (i, &b) in text.iter().enumerate().take(len) {
            let offset = base + i;
            // Preserve the cell's current color, as in `putstr`.
            let color = read_volatile(buf.add(offset)).color;
            let color = if color == 0 {
                COLOR_WHITE_ON_BLACK
            } else {
                color
            };
            write_volatile(buf.add(offset), ScreenCell { ascii: b, color });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_digits_zero_is_padded() {
        assert_eq!(hex_digits(0x0000_0000), *b"00000000");
    }

    #[test]
    fn hex_digits_typical_address() {
        // Typical `eip` address: mixed letters and digits.
        assert_eq!(hex_digits(0x1234_ABCD), *b"1234ABCD");
    }

    #[test]
    fn hex_digits_all_ones() {
        assert_eq!(hex_digits(0xFFFF_FFFF), *b"FFFFFFFF");
    }

    #[test]
    fn hex_digits_preserves_leading_zeros() {
        // Padding is load-bearing: without it `0xA` and `0xA0000000` would be ambiguous.
        assert_eq!(hex_digits(0x00F0_000A), *b"00F0000A");
    }

    #[test]
    fn hex_digits_uses_uppercase() {
        // Ensure uppercase `A..F`, not `a..f`.
        assert_eq!(hex_digits(0x00AB_CDEF), *b"00ABCDEF");
    }
}
