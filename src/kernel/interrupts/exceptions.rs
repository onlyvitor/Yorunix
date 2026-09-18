use crate::arch::x86::cpu::idt::InterruptFrame;
use crate::kernel::drivers::vga;
use core::arch::asm;
// #DE — Divide Error, vector 0, fault, no error code (ISR_NOERRCODE).
// Triggered by `div`/`idiv` with a zero divisor or a quotient that does not fit
// in the destination register (e.g. `mov ax, 0xFFFF / mov bl, 0 / div bl`).
// Fatal: `iret` would return to the same `eip` and redo the division in a loop,
// so print and HALT (`cli/hlt`) instead of returning. Never `ret` here.
pub extern "C" fn divide_error(frame: &InterruptFrame) {
    // Copy fields first: `frame` points to the ASM stack and the dump
    // below performs volatile MMIO; locals avoid surprising re-reads.
    let eip = frame.eip;
    let cs = frame.cs;
    let eflags = frame.eflags;

    // 2B positional layout (same as `debug`): `putstr` restarts at offset 0,
    // so labels in ONE call and values via `put_hex_at`.
    // Row 0: title / Row 1: "EIP=" / Row 2: "CS=" / Row 3: "EFLAGS=".
    vga::clear_screen();
    vga::putstr("Divide Error (#DE)!\nEIP=\nCS=\nEFLAGS=\n");
    // Col = label len: "EIP=" -> 4, "CS=" -> 3, "EFLAGS=" -> 7.
    vga::put_hex_at(eip, 1, 4);
    vga::put_hex_at(cs, 2, 3);
    vga::put_hex_at(eflags, 3, 7);
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

//#DB — vector 1, fault, no error code (ISR_NOERRCODE).
// Recoverable: print and RETURN (iret resumes). Never `cli/hlt` here,
// otherwise single-step / hardware breakpoint hangs the kernel.
pub fn debug(frame: &InterruptFrame) {
    // Copy fields first: `frame` points to the ASM stack and the dump
    // below performs volatile MMIO; locals avoid surprising re-reads.
    let eip = frame.eip;
    let cs = frame.cs;
    let eflags = frame.eflags;

    // 2B positional layout: `putstr` always restarts at offset 0, so
    // labels go in ONE call (with `\n`) and values via `put_hex_at`.
    // Row 0: title / Row 1: "EIP=" / Row 2: "CS=" / Row 3: "EFLAGS=".
    vga::clear_screen();
    vga::putstr("Debug Exception (#DB)!\nEIP=\nCS=\nEFLAGS=\n");
    // Col = label len: "EIP=" -> 4, "CS=" -> 3, "EFLAGS=" -> 7.
    vga::put_hex_at(eip, 1, 4);
    vga::put_hex_at(cs, 2, 3);
    vga::put_hex_at(eflags, 3, 7);
}

//NMI
pub fn no_maskable_interrupt(_frame: &InterruptFrame) {
    vga::putstr("No maskable interrupt!\n");
}

//#BP
pub fn breakpoint(_frame: &InterruptFrame) {
    vga::putstr("Breakpoint!\n");
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
