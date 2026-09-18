use crate::idt::InterruptFrame;
use crate::vga;
use core::arch::asm;
// #DE
pub extern "C" fn divide_error(_frame: &InterruptFrame) {
    vga::putstr("Divisao por zero!\n");
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

//#DB
pub fn debug(_frame: &InterruptFrame) {
    vga::putstr("InterruptFrame:\n");
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

// Reserved exceptions (32..255) — paridade com o C (no-op).
pub fn reserved(_frame: &InterruptFrame) {
    vga::putstr("Reserved exception!\n");
}
