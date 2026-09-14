//! IDT — Interrupt Descriptor Table (32-bit).
//!
//! Substitui `arch/x86/idt.c` + `idt.h` preservando o contrato com `idt.asm`:
//! - `i686_ISR_common` faz `pusha; mov ds/es/fs/gs,0x10; push esp; call
//!   i686_ISR_handler; add esp,4; popa; add esp,8; iret`.
//! - Por isso `InterruptFrame` replica exatamente essa ordem de empilhamento.
//!
//! Corrige bug do C original: `base_high` era `uint8_t` (gate de 7 bytes,
//! truncava 8 bits do handler). Aqui são 8 bytes com `base_high: u16`.

use core::mem::size_of;

pub const PRESENT: u8 = 0x80;
pub const RING0: u8 = 0x00;
pub const TYPE_INTERRUPT_GATE: u8 = 0x0E;
pub const SELECTOR_KERNEL_CODE: u16 = 0x08;

/// Gate de interrupção 32-bit — 8 bytes exatos exigidos pela CPU.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    base_low: u16,
    selector: u16,
    zero: u8,
    attr: u8,
    base_high: u16,
}

/// Registrador IDTR (limit u16 + base u32).
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtDescriptor {
    limit: u16,
    base: u32,
}

/// Deve espelhar `pusha` + `push int_num/error` + `eip/cs/eflags` da CPU.
///
/// Ordem do `pusha` no stack (ESP aponta para EDI após o `pusha`), depois
/// `int_num`, `error_code`, depois o que a CPU empilhou.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InterruptFrame {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub esp_save: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,
    pub int_num: u32,
    pub error_code: u32,
    pub eip: u32,
    pub cs: u32,
    pub eflags: u32,
}

const _: () = assert!(size_of::<IdtEntry>() == 8);
const _: () = assert!(size_of::<IdtDescriptor>() == 6);

static mut IDT: [IdtEntry; 256] = [IdtEntry {
    base_low: 0,
    selector: 0,
    zero: 0,
    attr: 0,
    base_high: 0,
}; 256];

// Stubs NASM — expandidos para 0..31 no `idt.asm` (ver fix ASM).
// Cada símbolo é o endereço do handler de baixo nível.
extern "C" {
    fn i686_ISR0();
    fn i686_ISR1();
    fn i686_ISR2();
    fn i686_ISR3();
    fn i686_ISR4();
    fn i686_ISR5();
    fn i686_ISR6();
    fn i686_ISR7();
    fn i686_ISR8();
    fn i686_ISR9();
    fn i686_ISR10();
    fn i686_ISR11();
    fn i686_ISR12();
    fn i686_ISR13();
    fn i686_ISR14();
    fn i686_ISR15();
    fn i686_ISR16();
    fn i686_ISR17();
    fn i686_ISR18();
    fn i686_ISR19();
    fn i686_ISR20();
    fn i686_ISR21();
    fn i686_ISR22();
    fn i686_ISR23();
    fn i686_ISR24();
    fn i686_ISR25();
    fn i686_ISR26();
    fn i686_ISR27();
    fn i686_ISR28();
    fn i686_ISR29();
    fn i686_ISR30();
    fn i686_ISR31();
}

fn set_gate(num: usize, base: u32, selector: u16, attr: u8) {
    // SAFETY: chamador garante `num < 256`. Campos escritos uma única vez no
    // boot com `cli`, antes de qualquer interrupção ser habilitada.
    unsafe {
        let entry = &mut *core::ptr::addr_of_mut!(IDT).cast::<[IdtEntry; 256]>();
        entry[num] = IdtEntry {
            base_low: (base & 0xFFFF) as u16,
            selector,
            zero: 0,
            attr,
            base_high: ((base >> 16) & 0xFFFF) as u16,
        };
    }
}

/// Handler genérico chamado pelo stub ASM. Paridade com o C (no-op).
/// Próximo passo sugerido: despachar por `int_num` e imprimir via VGA.
#[no_mangle]
pub extern "C" fn i686_ISR_handler(_frame: *mut InterruptFrame) {
    // Intencionalmente vazio nesta fase — mesma semântica do C `(void)frame`.
    // Não toca em `frame` para evitar page-fault em handler de exceção.
}

/// Ponto de entrada chamado pelo `boot/entry.asm`. Nome preservado.
#[no_mangle]
pub extern "C" fn idt_init() {
    const ATTR: u8 = PRESENT | RING0 | TYPE_INTERRUPT_GATE;
    // SAFETY: `IDT` vive por todo o kernel; `lidt` copia limit+base para o
    // IDTR. Endereços dos stubs vêm do linker (sempre válidos). Sem
    // concorrência no boot (`cli` no `_start`).
    unsafe {
        let handlers: [unsafe extern "C" fn(); 32] = [
            i686_ISR0, i686_ISR1, i686_ISR2, i686_ISR3, i686_ISR4, i686_ISR5,
            i686_ISR6, i686_ISR7, i686_ISR8, i686_ISR9, i686_ISR10, i686_ISR11,
            i686_ISR12, i686_ISR13, i686_ISR14, i686_ISR15, i686_ISR16,
            i686_ISR17, i686_ISR18, i686_ISR19, i686_ISR20, i686_ISR21,
            i686_ISR22, i686_ISR23, i686_ISR24, i686_ISR25, i686_ISR26,
            i686_ISR27, i686_ISR28, i686_ISR29, i686_ISR30, i686_ISR31,
        ];
        for (i, h) in handlers.iter().enumerate() {
            set_gate(i, *h as u32, SELECTOR_KERNEL_CODE, ATTR);
        }
        let desc = IdtDescriptor {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: core::ptr::addr_of!(IDT) as *const IdtEntry as u32,
        };
        core::arch::asm!("lidt [{}]", in(reg) &desc, options(nostack, preserves_flags));
    }
}
