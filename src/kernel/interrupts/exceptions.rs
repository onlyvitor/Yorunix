use crate::arch::x86::cpu::idt::InterruptFrame;
use crate::kernel::drivers::vga;
use core::arch::asm;

/// Parks the CPU forever: mask interrupts, then halt in a loop.
///
/// Fatal handlers call this after dumping state: for a fault, `iret` would
/// return to the faulting instruction and loop. Any NMI can still wake `hlt`
/// (it ignores `cli`), hence the loop re-halts. Same shape as the kernel
/// `panic` halt loop in `lib.rs`.
pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Columns where `put_hex_at` starts each value: always the length of its
/// label. Locked by tests so the dump layout cannot silently drift.
const COL_EIP: usize = 4; // len("EIP=")
const COL_CS: usize = 3; // len("CS=")
const COL_EFLAGS: usize = 7; // len("EFLAGS=")
const COL_ERR: usize = 4; // len("ERR=")

/// Fixed buffer for the dump's single `putstr` call (title + labels).
/// Longest title today ("SIMD floating point exception (#XF)!") is 36 bytes;
/// 80 leaves headroom. Overflow truncates, same policy as `putstr`.
const DUMP_BUF_LEN: usize = 80;

/// Pure composition of the dump's one `putstr` payload:
/// `title` + `\n` + labels (`ERR=` only for error-code vectors).
///
/// `putstr` always restarts at (0,0), so the title and all labels MUST be a
/// single string. No alloc/fmt (`no_std`): fixed buffer + manual byte copy,
/// deliberately avoiding `copy_from_slice` (it can lower to `memcpy`, see
/// vga.rs). Returns `(buffer, used_len)`; testable on the host.
fn build_dump_text(title: &str, show_error_code: bool) -> ([u8; DUMP_BUF_LEN], usize) {
    /// Pushes `b`, truncating silently when the buffer is full.
    fn push_byte(buf: &mut [u8; DUMP_BUF_LEN], len: &mut usize, b: u8) {
        if *len < DUMP_BUF_LEN {
            buf[*len] = b;
            *len += 1;
        }
    }

    const LABELS: &str = "EIP=\nCS=\nEFLAGS=\n";
    const LABELS_ERR: &str = "EIP=\nCS=\nEFLAGS=\nERR=\n";
    let labels = if show_error_code { LABELS_ERR } else { LABELS };
    let mut buf = [0u8; DUMP_BUF_LEN];
    let mut len = 0usize;
    for &b in title.as_bytes() {
        push_byte(&mut buf, &mut len, b);
    }
    // Separator between the title row and the first label row.
    push_byte(&mut buf, &mut len, b'\n');
    for &b in labels.as_bytes() {
        push_byte(&mut buf, &mut len, b);
    }
    (buf, len)
}

/// Shared exception dump: clears the screen, prints `title` on row 0, then
/// `EIP`, `CS`, `EFLAGS` (and `ERR` for vectors whose CPU pushes an error
/// code — the ASM dummy `0` elsewhere would be misleading).
///
/// Single output seam for all handlers; a serial sink later plugs in here
/// without touching individual handlers.
fn dump_exception(title: &str, frame: &InterruptFrame, show_error_code: bool) {
    // Copy fields first: `frame` points into the ASM stack while the dump
    // performs volatile MMIO; locals avoid surprising re-reads.
    let eip = frame.eip;
    let cs = frame.cs;
    let eflags = frame.eflags;
    let err = frame.error_code;

    vga::clear_screen();
    let (buf, len) = build_dump_text(title, show_error_code);
    // Bytes are pure ASCII by construction, so `from_utf8` cannot fail; the
    // fallback keeps the labels readable even if composition ever changes.
    let text = core::str::from_utf8(&buf[..len]).unwrap_or("EIP=\nCS=\nEFLAGS=\n");
    vga::putstr(text);
    vga::put_hex_at(eip, 1, COL_EIP);
    vga::put_hex_at(cs, 2, COL_CS);
    vga::put_hex_at(eflags, 3, COL_EFLAGS);
    if show_error_code {
        vga::put_hex_at(err, 4, COL_ERR);
    }
}

// #DE — Divide Error, vector 0, fault, no error code (ISR_NOERRCODE).
// Triggered by `div`/`idiv` with a zero divisor or a quotient that does not fit
// in the destination register (e.g. `mov ax, 0xFFFF / mov bl, 0 / div bl`).
// Fatal: `iret` would return to the same `eip` and redo the division in a
// loop, so dump and HALT. Never return here.
pub extern "C" fn divide_error(frame: &InterruptFrame) {
    dump_exception("Divide Error (#DE)!", frame, false);
    halt();
}

//#DB — vector 1, fault, no error code (ISR_NOERRCODE).
// Recoverable: dump and RETURN (`iret` resumes; single-step / hardware
// breakpoints keep running). Never `cli/hlt` here, otherwise single-step
// / hardware breakpoint hangs the kernel.
pub fn debug(frame: &InterruptFrame) {
    dump_exception("Debug Exception (#DB)!", frame, false);
}

//NMI
pub fn no_maskable_interrupt(_frame: &InterruptFrame) {
    vga::putstr("No maskable interrupt!\n");
}

//#BP vector 3 — Breakpoint, raised by the INT3 instruction.
// Traps after the instruction, so returning resumes normally; debug software
// plants INT3 as a breakpoint. Same dump shape as `debug` — and a title that
// names the right exception (it was copy-pasted as "#DB" before).
pub fn breakpoint(frame: &InterruptFrame) {
    dump_exception("Breakpoint Exception (#BP)!", frame, false);
}

//#OF
pub fn overflow(_frame: &InterruptFrame) {
    vga::putstr("Overflow!\n");
}

//#BR
pub fn bound_range_exceeded(_frame: &InterruptFrame) {
    vga::putstr("Bound range exceeded!\n");
}

//#UD
pub fn invalid_opcode(_frame: &InterruptFrame) {
    vga::putstr("Invalid opcode!\n");
}
//#NM
pub fn device_not_available(_frame: &InterruptFrame) {
    vga::putstr("Device not available!\n");
}

//#DF
pub fn double_fault(_frame: &InterruptFrame) {
    vga::putstr("Double fault!\n");
}

//CSO
pub fn coprocessor_segment_overrun(_frame: &InterruptFrame) {
    vga::putstr("Coprocessor segment overrun!\n");
}

//TS
pub fn invalid_tss(_frame: &InterruptFrame) {
    vga::putstr("Invalid TSS!\n");
}

//#NP
pub fn segment_not_present(_frame: &InterruptFrame) {
    vga::putstr("Segment not present!\n");
}

//#SS
pub fn stack_segment_fault(_frame: &InterruptFrame) {
    vga::putstr("Stack segment fault!\n");
}

//#GP
pub fn general_protection_fault(_frame: &InterruptFrame) {
    vga::putstr("General protection fault!\n");
}

//#PF
pub fn page_fault(_frame: &InterruptFrame) {
    vga::putstr("Page fault!\n");
}

//MF
pub fn floating_point_error(_frame: &InterruptFrame) {
    vga::putstr("Floating point error!\n");
}

//#AC
pub fn alignment_check(_frame: &InterruptFrame) {
    vga::putstr("Alignment check!\n");
}

//#MC
pub fn machine_check(_frame: &InterruptFrame) {
    vga::putstr("Machine check!\n");
}

//#XM/XF
pub fn simd_floating_point(_frame: &InterruptFrame) {
    vga::putstr("SIMD floating point exception!\n");
}

//#VE
pub fn virtualization(_frame: &InterruptFrame) {
    vga::putstr("Virtualization exception!\n");
}

//#SX
pub fn security_exception(_frame: &InterruptFrame) {
    vga::putstr("Security exception!\n");
}

// Reserved exceptions (32..255) — parity with C (no-op).
pub fn reserved(_frame: &InterruptFrame) {
    vga::putstr("Reserved exception!\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the pure composition and returns the payload as a string.
    fn text(title: &str, show_error_code: bool) -> String {
        let (buf, len) = build_dump_text(title, show_error_code);
        String::from_utf8_lossy(&buf[..len]).into_owned()
    }

    #[test]
    fn dump_text_without_error_code() {
        assert_eq!(
            text("Divide Error (#DE)!", false),
            "Divide Error (#DE)!\nEIP=\nCS=\nEFLAGS=\n"
        );
    }

    #[test]
    fn dump_text_with_error_code() {
        assert_eq!(
            text("General Protection Fault (#GP)!", true),
            "General Protection Fault (#GP)!\nEIP=\nCS=\nEFLAGS=\nERR=\n"
        );
    }

    #[test]
    fn dump_text_truncates_overlong_title() {
        // Must not panic and must fill exactly the buffer capacity.
        let (_, len) = build_dump_text(&"x".repeat(200), false);
        assert_eq!(len, DUMP_BUF_LEN);
    }

    #[test]
    fn columns_match_label_lengths() {
        // `put_hex_at` starts each value right after its label; these
        // constants are the layout contract between `build_dump_text`
        // labels and the `put_hex_at` call sites.
        assert_eq!(COL_EIP, "EIP=".len());
        assert_eq!(COL_CS, "CS=".len());
        assert_eq!(COL_EFLAGS, "EFLAGS=".len());
        assert_eq!(COL_ERR, "ERR=".len());
    }
}
