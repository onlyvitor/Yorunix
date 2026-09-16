use crate::vga;
use crate::idt::InterruptFrame;

//#DE
pub fn divide_error(frame: &InterruptFrame) {
        vga::putstr("Divisão por zero!\n");
        frame.int_num;
}

//#DB
pub fn debug(frame: &InterruptFrame) {
        vga::putstr("InterruptFrame:\n");
        frame.int_num;
}

//NMI
pub fn no_maskable_interrupt(frame: &InterruptFrame) {
        vga::putstr("No maskable interrupt!\n");
        frame.int_num;
}

//#BP
pub fn breakpoint(frame: &InterruptFrame) {
        vga::putstr("Breakpoint!\n");
        frame.int_num;
}

//#OF
pub fn overflow(frame: &InterruptFrame) {
        vga::putstr("Overflow!\n");
        frame.int_num;
}

//#BR
pub fn bound_range_exceeded(frame: &InterruptFrame) {
        vga::putstr("Bound range exceeded!\n");
        frame.int_num;
}

//#UD
pub fn invalid_opcode(frame: &InterruptFrame) {
        vga::putstr("Invalid opcode!\n");
        frame.int_num;
}
//#NM
pub fn device_not_available(frame: &InterruptFrame) {
        vga::putstr("Device not available!\n");
        frame.int_num;
}

//#DF
pub fn double_fault(frame: &InterruptFrame) {
        vga::putstr("Double fault!\n");
        frame.int_num;
}

//CSO
pub fn coprocessor_segment_overrun(frame: &InterruptFrame) {
        vga::putstr("Coprocessor segment overrun!\n");
        frame.int_num;
}

//TS
pub fn invalid_tss(frame: &InterruptFrame) {
        vga::putstr("Invalid TSS!\n");
        frame.int_num;
}

//#NP
pub fn segment_not_present(frame: &InterruptFrame) {
        vga::putstr("Segment not present!\n");
        frame.int_num;
}

//#SS
pub fn stack_segment_fault(frame: &InterruptFrame) {
        vga::putstr("Stack segment fault!\n");
        frame.int_num;
}

//#GP
pub fn general_protection_fault(frame: &InterruptFrame) {
        vga::putstr("General protection fault!\n");
        frame.int_num;
}

//#PF
pub fn page_fault(frame: &InterruptFrame) {
        vga::putstr("Page fault!\n");
        frame.int_num;
}

//MF
pub fn floating_point_error(frame: &InterruptFrame) {
        vga::putstr("Floating point error!\n");
        frame.int_num;
}

//#AC
pub fn alignment_check(frame: &InterruptFrame) {
        vga::putstr("Alignment check!\n");
        frame.int_num;
}

//#MC
pub fn machine_check(frame: &InterruptFrame) {
        vga::putstr("Machine check!\n");
        frame.int_num;
}

//#XM/XF
pub fn simd_floating_point(frame: &InterruptFrame) {
        vga::putstr("SIMD floating point exception!\n");
        frame.int_num;
}


//#VE
pub fn virtualization(frame: &InterruptFrame) {
        vga::putstr("Virtualization exception!\n");
        frame.int_num;
}

//#SX
pub fn security_exception(frame: &InterruptFrame) {
        vga::putstr("Security exception!\n");
        frame.int_num;
}

// Reserved exceptions (32..255) — paridade com o C (no-op).
pub fn reserved(frame: &InterruptFrame) {
        vga::putstr("Reserved exception!\n");
        frame.int_num;
}