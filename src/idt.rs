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
// No `cargo test` (host) os .o do NASM não participam do link, então stubs
// no-op com os mesmos símbolos satisfazem o linker; `idt_init` nunca é
// chamado nos testes.
#[cfg(not(test))]
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

#[cfg(test)]
macro_rules! define_test_isr_stubs {
    ($($name:ident),*) => {
        $(
            // Nomes replicam os símbolos ASM de propósito.
            #[allow(dead_code, non_snake_case)]
            unsafe extern "C" fn $name() {}
        )*
    };
}

#[cfg(test)]
define_test_isr_stubs!(
    i686_ISR0, i686_ISR1, i686_ISR2, i686_ISR3, i686_ISR4, i686_ISR5, i686_ISR6, i686_ISR7,
    i686_ISR8, i686_ISR9, i686_ISR10, i686_ISR11, i686_ISR12, i686_ISR13, i686_ISR14, i686_ISR15,
    i686_ISR16, i686_ISR17, i686_ISR18, i686_ISR19, i686_ISR20, i686_ISR21, i686_ISR22, i686_ISR23,
    i686_ISR24, i686_ISR25, i686_ISR26, i686_ISR27, i686_ISR28, i686_ISR29, i686_ISR30, i686_ISR31
);

/// Construtor puro de gate — mesma codificação usada no boot, sem tocar na
/// tabela global, para ser unitariamente testável no host.
const fn make_gate(base: u32, selector: u16, attr: u8) -> IdtEntry {
    IdtEntry {
        base_low: (base & 0xFFFF) as u16,
        selector,
        zero: 0,
        attr,
        base_high: ((base >> 16) & 0xFFFF) as u16,
    }
}

fn set_gate(num: usize, base: u32, selector: u16, attr: u8) {
    // SAFETY: chamador garante `num < 256`. Campos escritos uma única vez no
    // boot com `cli`, antes de qualquer interrupção ser habilitada.
    unsafe {
        let entry = &mut *core::ptr::addr_of_mut!(IDT).cast::<[IdtEntry; 256]>();
        entry[num] = make_gate(base, selector, attr);
    }
}

/// Vetor do divide error (#DE). Constante evita magic number no dispatch.
pub const DIVIDE_VECTOR: u32 = 0;

/// Vetor da exceção de debug (#DB). Mantido como constante para evitar
/// magic number no dispatch e facilitar testes no host.
pub const DEBUG_VECTOR: u32 = 1;

/// Handler genérico chamado pelo stub ASM (`push esp; call i686_ISR_handler`).
/// Despacha por `int_num`. Vetores 0 (#DE) e 1 (#DB) têm handler real;
/// os demais seguem no-op para não mudar comportamento dos outros vetores.
#[no_mangle]
// Não pode ser `unsafe fn`: é chamado via `call` direto do stub NASM.
// O lint é suprimido localmente; a segurança é justificada no bloco abaixo.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn i686_ISR_handler(frame: *mut InterruptFrame) {
    // SAFETY: o stub ASM sempre passa `esp` (ponteiro válido do frame).
    // Checagem de nulo por defesa: em QEMU/host-test um ponteiro nulo
    // jamais deve derrubar o kernel dentro do handler.
    unsafe {
        if frame.is_null() {
            return;
        }
        let f = &*frame;
        if f.int_num == DIVIDE_VECTOR {
            crate::handler::idt_handler::divide_error(f);
        } else if f.int_num == DEBUG_VECTOR {
            crate::handler::idt_handler::debug(f);
        }
    }
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
            i686_ISR0, i686_ISR1, i686_ISR2, i686_ISR3, i686_ISR4, i686_ISR5, i686_ISR6, i686_ISR7,
            i686_ISR8, i686_ISR9, i686_ISR10, i686_ISR11, i686_ISR12, i686_ISR13, i686_ISR14,
            i686_ISR15, i686_ISR16, i686_ISR17, i686_ISR18, i686_ISR19, i686_ISR20, i686_ISR21,
            i686_ISR22, i686_ISR23, i686_ISR24, i686_ISR25, i686_ISR26, i686_ISR27, i686_ISR28,
            i686_ISR29, i686_ISR30, i686_ISR31,
        ];
        for (i, h) in handlers.iter().enumerate() {
            // No alvo real (i686) um `u32` cobre todo endereço — não há
            // truncamento. O warning de truncamento só existe no host
            // 64-bit (onde `idt_init` é compilado mas nunca chamado), e a
            // variante sem truncamento dispara no i686.
            #[allow(clippy::fn_to_numeric_cast, clippy::fn_to_numeric_cast_with_truncation)]
            let addr = *h as u32;
            set_gate(i, addr, SELECTOR_KERNEL_CODE, ATTR);
        }
        let desc = IdtDescriptor {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: core::ptr::addr_of!(IDT) as *const IdtEntry as u32,
        };
        core::arch::asm!("lidt [{}]", in(reg) &desc, options(nostack, preserves_flags));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ver nota em gdt::tests: campos packed são copiados, nunca referenciados.
    fn fields(g: IdtEntry) -> (u16, u16, u8, u8, u16) {
        (g.base_low, g.selector, g.zero, g.attr, g.base_high)
    }

    #[test]
    fn gate_splits_handler_address() {
        let g = make_gate(
            0x12345678,
            SELECTOR_KERNEL_CODE,
            PRESENT | RING0 | TYPE_INTERRUPT_GATE,
        );
        // attr = 0x80 | 0x0E = 0x8E; base 0x12345678 -> low 0x5678, high 0x1234.
        assert_eq!(fields(g), (0x5678, 0x08, 0, 0x8E, 0x1234));
    }

    #[test]
    fn null_gate_is_zeroed() {
        assert_eq!(fields(make_gate(0, 0, 0)), (0, 0, 0, 0, 0));
    }

    #[test]
    fn debug_vector_is_one() {
        // #DB é o vetor 1 (Intel SDM Vol.3 Ch.6). Trava o contrato do dispatch.
        assert_eq!(DEBUG_VECTOR, 1);
    }

    #[test]
    fn divide_vector_is_zero() {
        // #DE é o vetor 0. Trava o contrato do dispatch.
        assert_eq!(DIVIDE_VECTOR, 0);
    }

    #[test]
    fn handler_ignores_null_frame() {
        // Não deve travar nem tocar em MMIO com ponteiro nulo.
        i686_ISR_handler(core::ptr::null_mut());
    }
}
