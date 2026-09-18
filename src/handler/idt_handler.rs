use crate::idt::InterruptFrame;
use crate::vga;
use core::arch::asm;
// #DE — Divide Error, vector 0, fault, sem error code (ISR_NOERRCODE).
// Dispara em `div`/`idiv` com divisor 0 ou quociente que não cabe no
// registrador destino (ex.: `mov ax, 0xFFFF / mov bl, 0 / div bl`).
// Fatal: `iret` retornaria ao mesmo `eip` e refaria a divisão em loop,
// então imprime e TRAVA (`cli/hlt`) em vez de retornar. Nunca `ret` aqui.
pub extern "C" fn divide_error(frame: &InterruptFrame) {
    // Copia campos primeiro: `frame` aponta para o stack do ASM e o dump
    // abaixo faz MMIO volátil; locais evitam releituras surpreendentes.
    let eip = frame.eip;
    let cs = frame.cs;
    let eflags = frame.eflags;

    // Layout posicional 2B (igual ao `debug`): `putstr` recomeça no offset 0,
    // então rótulos em UMA chamada e valores via `put_hex_at`.
    // Linha 0: título / Linha 1: "EIP=" / Linha 2: "CS=" / Linha 3: "EFLAGS=".
    vga::clear_screen();
    vga::putstr("Divide Error (#DE)!\nEIP=\nCS=\nEFLAGS=\n");
    // Col = len do rótulo: "EIP=" -> 4, "CS=" -> 3, "EFLAGS=" -> 7.
    vga::put_hex_at(eip, 1, 4);
    vga::put_hex_at(cs, 2, 3);
    vga::put_hex_at(eflags, 3, 7);
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

//#DB — vector 1, fault, sem error code (ISR_NOERRCODE).
// Recoverable: imprime e RETORNA (iret resume). Nunca `cli/hlt` aqui,
// senão single-step / hardware breakpoint trava o kernel.
pub fn debug(frame: &InterruptFrame) {
    // Copia campos primeiro: `frame` aponta para o stack do ASM e o dump
    // abaixo faz MMIO volátil; locais evitam releituras surpreendentes.
    let eip = frame.eip;
    let cs = frame.cs;
    let eflags = frame.eflags;

    // Layout posicional 2B: `putstr` sempre recomeça no offset 0, então os
    // rótulos vão em UMA chamada (com `\n`) e os valores via `put_hex_at`.
    // Linha 0: título / Linha 1: "EIP=" / Linha 2: "CS=" / Linha 3: "EFLAGS=".
    vga::clear_screen();
    vga::putstr("Debug Exception (#DB)!\nEIP=\nCS=\nEFLAGS=\n");
    // Col = len do rótulo: "EIP=" -> 4, "CS=" -> 3, "EFLAGS=" -> 7.
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

// Reserved exceptions (32..255) — paridade com o C (no-op).
pub fn reserved(_frame: &InterruptFrame) {
    vga::putstr("Reserved exception!\n");
}
