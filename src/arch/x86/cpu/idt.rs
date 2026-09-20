//! IDT — Interrupt Descriptor Table (32-bit).
//!
//! Replaces `arch/x86/idt.c` + `idt.h` while preserving the contract with `idt.asm`:
//! - `i686_ISR_common` does `pusha; mov ds/es/fs/gs,0x10; save esp; realign;
//!   call i686_ISR_handler(frame); restore esp; popa; add esp,8; iret` — the
//!   realignment gives the Rust handler the SysV 16-byte stack alignment
//!   LLVM-emitted SSE assumes (see idt.asm).
//! - Hence `InterruptFrame` replicates exactly that push order.
//!
//! Fixes a bug in the original C: `base_high` was `uint8_t` (a 7-byte gate,
//! truncating 8 bits of the handler). Here it is 8 bytes with `base_high: u16`.

use core::mem::size_of;

use crate::kernel::drivers::vga;

pub const PRESENT: u8 = 0x80;
pub const RING0: u8 = 0x00;
pub const TYPE_INTERRUPT_GATE: u8 = 0x0E;
pub const SELECTOR_KERNEL_CODE: u16 = 0x08;

/// 32-bit interrupt gate — exact 8 bytes required by the CPU.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    base_low: u16,
    selector: u16,
    zero: u8,
    attr: u8,
    base_high: u16,
}

/// IDTR register (limit u16 + base u32).
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtDescriptor {
    limit: u16,
    base: u32,
}

/// Must mirror `pusha` + `push int_num/error` + CPU `eip/cs/eflags`.
///
/// `pusha` order on the stack (ESP points to EDI after `pusha`), then
/// `int_num`, `error_code`, then what the CPU pushed.
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

// NASM stubs — expanded to 0..31 in `idt.asm` (see ASM fix).
// Each symbol is the address of the low-level handler.
// Under `cargo test` (host) the NASM .o files do not take part in linking, so
// no-op stubs with the same symbols satisfy the linker; `idt_init` is never
// called in tests.
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
            // Names intentionally replicate the ASM symbols.
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

/// Pure gate constructor — same encoding used at boot, without touching the
/// global table, so it is unit-testable on the host.
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
    // SAFETY: caller guarantees `num < 256`. Fields written a single time at
    // boot with `cli`, before any interrupt is enabled.
    unsafe {
        let entry = &mut *core::ptr::addr_of_mut!(IDT).cast::<[IdtEntry; 256]>();
        entry[num] = make_gate(base, selector, attr);
    }
}

/// Divide error vector (#DE). Constant avoids a magic number in dispatch.
pub const DIVIDE_VECTOR: u32 = 0;

/// Debug exception vector (#DB). Kept as a constant to avoid a
/// magic number in dispatch and to ease testing on the host.
pub const DEBUG_VECTOR: u32 = 1;

/// Breakpoint exception vector. keep constant
pub const BREAKPOINT_VECTOR: u32 = 3;

/// Generic handler called by the ASM stub (`push esp; call i686_ISR_handler`).
/// Dispatches on `int_num`. Vectors migrate to real handlers group by group
/// (per-vector policy lives in `exceptions.rs`); unwired ones still fall
/// through to a "not implemented yet" message.
#[no_mangle]
// Cannot be an `unsafe fn`: it is called via a direct `call` from the NASM stub.
// The lint is suppressed locally; safety is justified in the block below.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn i686_ISR_handler(frame: *mut InterruptFrame) {
    // SAFETY: the ASM trampoline always passes the saved frame pointer
    // (`esp` at exception entry). Defensive null check: under QEMU/host-test
    // a null pointer must never bring down the kernel inside the handler.
    unsafe {
        if frame.is_null() {
            return;
        }
        let f = &*frame;
        match f.int_num {
            DIVIDE_VECTOR => crate::kernel::interrupts::exceptions::divide_error(f),
            DEBUG_VECTOR => crate::kernel::interrupts::exceptions::debug(f),
            BREAKPOINT_VECTOR => crate::kernel::interrupts::exceptions::breakpoint(f),
            4 => crate::kernel::interrupts::exceptions::overflow(f),
            5 => crate::kernel::interrupts::exceptions::bound_range_exceeded(f),
            6 => crate::kernel::interrupts::exceptions::invalid_opcode(f),
            7 => crate::kernel::interrupts::exceptions::device_not_available(f),
            8 => crate::kernel::interrupts::exceptions::double_fault(f),
            9 => crate::kernel::interrupts::exceptions::coprocessor_segment_overrun(f),
            10 => crate::kernel::interrupts::exceptions::invalid_tss(f),
            11 => crate::kernel::interrupts::exceptions::segment_not_present(f),
            12 => crate::kernel::interrupts::exceptions::stack_segment_fault(f),
            13 => crate::kernel::interrupts::exceptions::general_protection_fault(f),
            14 => crate::kernel::interrupts::exceptions::page_fault(f),
            16 => crate::kernel::interrupts::exceptions::floating_point_error(f),
            17 => crate::kernel::interrupts::exceptions::alignment_check(f),
            18 => crate::kernel::interrupts::exceptions::machine_check(f),
            19 => crate::kernel::interrupts::exceptions::simd_floating_point(f),
            20 => crate::kernel::interrupts::exceptions::virtualization(f),
            21 => crate::kernel::interrupts::exceptions::control_protection(f),
            30 => crate::kernel::interrupts::exceptions::security_exception(f),
            _ => {
                vga::putstr("not implemented yet");
            }
        }
    }
}

/// Entry point called by `arch/x86/boot/entry.asm`. Name preserved.
#[no_mangle]
pub extern "C" fn idt_init() {
    const ATTR: u8 = PRESENT | RING0 | TYPE_INTERRUPT_GATE;
    // SAFETY: `IDT` lives for the whole kernel; `lidt` copies limit+base into
    // the IDTR. Stub addresses come from the linker (always valid). No
    // concurrency at boot (`cli` in `_start`).
    unsafe {
        let handlers: [unsafe extern "C" fn(); 32] = [
            i686_ISR0, i686_ISR1, i686_ISR2, i686_ISR3, i686_ISR4, i686_ISR5, i686_ISR6, i686_ISR7,
            i686_ISR8, i686_ISR9, i686_ISR10, i686_ISR11, i686_ISR12, i686_ISR13, i686_ISR14,
            i686_ISR15, i686_ISR16, i686_ISR17, i686_ISR18, i686_ISR19, i686_ISR20, i686_ISR21,
            i686_ISR22, i686_ISR23, i686_ISR24, i686_ISR25, i686_ISR26, i686_ISR27, i686_ISR28,
            i686_ISR29, i686_ISR30, i686_ISR31,
        ];
        for (i, h) in handlers.iter().enumerate() {
            // On the real target (i686) a `u32` covers every address — no
            // truncation. The truncation warning only exists on the 64-bit
            // host (where `idt_init` is compiled but never called), and the
            // non-truncating variant fires on i686.
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

    // See note in crate::arch::x86::cpu::gdt::tests: packed fields are copied, never referenced.
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
        // #DB is vector 1 (Intel SDM Vol.3 Ch.6). Locks the dispatch contract.
        assert_eq!(DEBUG_VECTOR, 1);
    }

    #[test]
    fn divide_vector_is_zero() {
        // #DE is vector 0. Locks the dispatch contract.
        assert_eq!(DIVIDE_VECTOR, 0);
    }

    #[test]
    fn handler_ignores_null_frame() {
        // Must not hang nor touch MMIO with a null pointer.
        i686_ISR_handler(core::ptr::null_mut());
    }
}
