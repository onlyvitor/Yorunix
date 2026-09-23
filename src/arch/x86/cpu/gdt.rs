//! GDT — Global Descriptor Table (32-bit, ring 0).
//!
//! Replaces `arch/x86/gdt.c` + `gdt.h` while preserving the `gdt.asm` ABI:
//! - `#[repr(C, packed)]` layout identical to C `__attribute__((packed))`.
//! - `i686_GDT_Initialize` symbol with the same `extern "C" fn()` signature.
//! - Calls `i686_GDT_Load(desc, 0x08, 0x10)` implemented in NASM
//!   (`lgdt` + far `retf` to reload CS + `mov ds/es/fs/gs/ss`).

use core::mem::size_of;

pub const CODE_SEGMENT: u16 = 0x08;
pub const DATA_SEGMENT: u16 = 0x10;

// --- Access byte (same values as the GDT_ACCESS enum in gdt.c) ---
pub const ACCESS_PRESENT: u8 = 0x80;
pub const ACCESS_RING0: u8 = 0x00;
pub const ACCESS_CODE_SEGMENT: u8 = 0x18;
pub const ACCESS_DATA_SEGMENT: u8 = 0x10;
pub const ACCESS_CODE_READABLE: u8 = 0x02;
pub const ACCESS_DATA_WRITEABLE: u8 = 0x02;

// --- Flags + high limit nibble (same values as GDT_FLAGS) ---
pub const FLAG_32BIT: u8 = 0x40;
pub const FLAG_GRANULARITY_4K: u8 = 0x80;

/// 8-byte GDT entry. Must remain `packed` for `lgdt`.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    flags_limit_hi: u8,
    base_high: u8,
}

/// 6-byte GDTR descriptor (limit u16 + base u32).
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtDescriptor {
    limit: u16,
    base: u32,
}

const _: () = assert!(size_of::<GdtEntry>() == 8);
const _: () = assert!(size_of::<GdtDescriptor>() == 6);

impl GdtEntry {
    /// `const` equivalent of the C `GDT_ENTRY(base, limit, access, flags)` macro.
    pub const fn new(base: u32, limit: u32, access: u8, flags: u8) -> Self {
        Self {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            flags_limit_hi: ((((limit >> 16) & 0x0F) as u8) | (flags & 0xF0)),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }
}

// Table live while the kernel runs — hence `static mut` + access in
// `init` with interrupts disabled (`_start` issues `cli` before calling).
static mut GDT: [GdtEntry; 3] = [
    // NULL descriptor
    GdtEntry::new(0, 0, 0, 0),
    // Kernel code 32-bit: base 0, limit 0xFFFFF, 4K granularity (= 4GB)
    GdtEntry::new(
        0,
        0xFFFFF,
        ACCESS_PRESENT | ACCESS_RING0 | ACCESS_CODE_SEGMENT | ACCESS_CODE_READABLE,
        FLAG_32BIT | FLAG_GRANULARITY_4K,
    ),
    // Kernel data 32-bit
    GdtEntry::new(
        0,
        0xFFFFF,
        ACCESS_PRESENT | ACCESS_RING0 | ACCESS_DATA_SEGMENT | ACCESS_DATA_WRITEABLE,
        FLAG_32BIT | FLAG_GRANULARITY_4K,
    ),
];

#[cfg(not(test))]
extern "C" {
    fn i686_GDT_Load(desc: *const GdtDescriptor, code: u16, data: u16);
}

/// Test stub: there is no GDTR on the host; it only allows linking and validating the call.
// The name intentionally replicates the ASM symbol.
#[cfg(test)]
#[allow(non_snake_case)]
unsafe fn i686_GDT_Load(_desc: *const GdtDescriptor, _code: u16, _data: u16) {}

/// Entry point called by `arch/x86/boot/entry.asm`.
/// Keeps the exact C name/symbol so the ASM stays untouched.
#[no_mangle]
pub extern "C" fn i686_GDT_Initialize() {
    // SAFETY: `GDT` is static and lives for the whole kernel; we build the
    // descriptor on the stack (`lgdt` copies limit+base into the GDTR, so the
    // temporary does not need to outlive the call). Called once at boot with `cli`,
    // without concurrency. The ASM function reloads CS via far-ret and the data
    // segments.
    unsafe {
        let desc = GdtDescriptor {
            limit: (size_of::<[GdtEntry; 3]>() - 1) as u16,
            base: core::ptr::addr_of!(GDT) as *const GdtEntry as u32,
        };
        i686_GDT_Load(&desc, CODE_SEGMENT, DATA_SEGMENT);
    }
}

/// Verifies the loaded GDTR against the static table: `sgdt` reads back
/// what `lgdt` stored and we compare limit and base with what
/// `i686_GDT_Initialize` put there. Runs after init (entry.asm boot order).
pub fn check() -> bool {
    let mut desc = GdtDescriptor { limit: 0, base: 0 };
    unsafe {
        // SAFETY: `sgdt` writes 6 bytes (limit + base) into the packed
        // struct and preserves flags. Kernel-only: on the 64-bit test host
        // the instruction would write 10 bytes, but `check` is never called
        // there (it compiles, it does not run).
        core::arch::asm!(
            "sgdt [{}]",
            in(reg) core::ptr::addr_of_mut!(desc),
            options(preserves_flags)
        );
    }
    // Packed fields cannot be borrowed (E0793): copy to locals first.
    let (limit, base) = (desc.limit, desc.base);
    let expected_limit = (size_of::<[GdtEntry; 3]>() - 1) as u16;
    let expected_base = core::ptr::addr_of!(GDT) as u32;
    limit == expected_limit && base == expected_base
}

#[cfg(test)]
mod tests {
    use super::*;

    // `assert_eq!` borrows its operands, and packed struct fields cannot
    // be referenced (E0793). We copy them to locals before comparing.
    fn fields(e: GdtEntry) -> (u16, u16, u8, u8, u8, u8) {
        (
            e.limit_low,
            e.base_low,
            e.base_middle,
            e.access,
            e.flags_limit_hi,
            e.base_high,
        )
    }

    #[test]
    fn null_descriptor_is_zeroed() {
        assert_eq!(fields(GdtEntry::new(0, 0, 0, 0)), (0, 0, 0, 0, 0, 0));
    }

    #[test]
    fn kernel_code_segment_encoding() {
        let e = GdtEntry::new(
            0,
            0xFFFFF,
            ACCESS_PRESENT | ACCESS_RING0 | ACCESS_CODE_SEGMENT | ACCESS_CODE_READABLE,
            FLAG_32BIT | FLAG_GRANULARITY_4K,
        );
        // access = 0x80 | 0x18 | 0x02 = 0x9A; high limit 0xF + flags 0xC0 = 0xCF.
        assert_eq!(fields(e), (0xFFFF, 0, 0, 0x9A, 0xCF, 0));
    }

    #[test]
    fn kernel_data_segment_encoding() {
        let e = GdtEntry::new(
            0,
            0xFFFFF,
            ACCESS_PRESENT | ACCESS_RING0 | ACCESS_DATA_SEGMENT | ACCESS_DATA_WRITEABLE,
            FLAG_32BIT | FLAG_GRANULARITY_4K,
        );
        // access = 0x80 | 0x10 | 0x02 = 0x92.
        assert_eq!(fields(e), (0xFFFF, 0, 0, 0x92, 0xCF, 0));
    }

    #[test]
    fn base_and_limit_fields_split_correctly() {
        let e = GdtEntry::new(0x12345678, 0xABCDE, 0, 0);
        assert_eq!(fields(e), (0xBCDE, 0x5678, 0x34, 0, 0x0A, 0x12));
    }
}
