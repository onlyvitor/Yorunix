//! CPU exception handlers (vectors 0–31) — the policy layer under the
//! `i686_ISR_handler` dispatch in `arch/x86/cpu/idt.rs`.
//!
//! Per-vector policy, fixed once and reused by every handler:
//!
//! - **Return only when resuming is meaningful: #DB (1) and #BP (3).**
//!   These traps resume past the instruction, which is what single-step and
//!   `int3` debugging need.
//! - **Everything else: dump + halt.** For a fault, `iret` re-executes the
//!   faulting instruction and loops — there is no paging, no user mode, and
//!   no consumer of `into`/`bound`/FPU to recover into.
//! - **`ERR=` only for the CPU error-code group** (8, 10–14, 17, 30);
//!   elsewhere the NASM stub pushes a dummy `0` that would mislead.
//! - Known limitation: #DF (8) runs on the same stack that may have caused
//!   the fault — the dump itself can fault again (triple fault). Fixing it
//!   needs a task gate / TSS (plan.md Phase 5).
//!
//! Output goes through `dump_exception` (VGA today; a serial sink later plugs
//! in there without touching handlers); fatal handlers end in `halt`.

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

//#OF vector 4 — Overflow, trap raised by INTO when OF=1.
// The kernel never executes INTO, so reaching here means something deeply
// unexpected: dump and halt instead of resuming into unknown state.
pub fn overflow(frame: &InterruptFrame) {
    dump_exception("Overflow Exception (#OF)!", frame, false);
    halt();
}

//#BR vector 5 — Bound Range Exceeded, raised by BOUND with an out-of-range index.
// No consumer of BOUND exists; dump and halt.
pub fn bound_range_exceeded(frame: &InterruptFrame) {
    dump_exception("Bound Range Exceeded (#BR)!", frame, false);
    halt();
}

//#UD vector 6 — Invalid Opcode, fault on an undefined/invalid instruction.
// Fatal: `iret` re-executes the same invalid instruction in a loop.
pub fn invalid_opcode(frame: &InterruptFrame) {
    dump_exception("Invalid Opcode (#UD)!", frame, false);
    halt();
}

//#NM vector 7 — Device Not Available, fault on x87 use with CR0.EM/TS set.
// Returning would re-execute the FPU instruction; proper lazy FPU handling
// is far future, so dump and halt.
pub fn device_not_available(frame: &InterruptFrame) {
    dump_exception("Device Not Available (#NM)!", frame, false);
    halt();
}

//#DF
pub fn double_fault(_frame: &InterruptFrame) {
    vga::putstr("Double fault!\n");
}

//#CSO vector 9 — Coprocessor Segment Overrun (legacy 286/386 fault).
// Fatal; no recovery path exists on modern hardware anyway.
pub fn coprocessor_segment_overrun(frame: &InterruptFrame) {
    dump_exception("Coprocessor Segment Overrun (#CSO)!", frame, false);
    halt();
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

//#MF vector 16 — x87 FPU Error (fired on FWAIT/FNINIT when CR0.NE=0).
// Fatal; `iret` would re-execute the x87 instruction.
pub fn floating_point_error(frame: &InterruptFrame) {
    dump_exception("x87 Floating Point Exception (#MF)!", frame, false);
    halt();
}

//#AC
pub fn alignment_check(_frame: &InterruptFrame) {
    vga::putstr("Alignment check!\n");
}

//#MC vector 18 — Machine Check, abort-class hardware error.
// State is not trustworthy: dump what is safe and halt.
pub fn machine_check(frame: &InterruptFrame) {
    dump_exception("Machine Check (#MC)!", frame, false);
    halt();
}

//#XM/XF vector 19 — SIMD Floating Point Exception (SSE fault).
// Fatal; `iret` would re-execute the SSE instruction.
pub fn simd_floating_point(frame: &InterruptFrame) {
    dump_exception("SIMD Floating Point Exception (#XM)!", frame, false);
    halt();
}

//#VE vector 20 — Virtualization Exception (requires VMX; cannot fire on this
// bare-metal/legacy target, but the gate is wired for completeness).
pub fn virtualization(frame: &InterruptFrame) {
    dump_exception("Virtualization Exception (#VE)!", frame, false);
    halt();
}

//#CP vector 21 — Control Protection (CET: shadow stack / IBT violations).
// NOTE: the NASM stub keeps this vector in the NOERR group (idt.asm), so an
// error code pushed by a real CET event would misalign this frame. CET does
// not exist on the i686 target, and `int $21` pushes no error code, so the
// dump stays consistent. See the annotation in idt.asm.
pub fn control_protection(frame: &InterruptFrame) {
    dump_exception("Control Protection Exception (#CP)!", frame, false);
    halt();
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
