//! GDT — Global Descriptor Table (32-bit, anel 0).
//!
//! Substitui `arch/x86/gdt.c` + `gdt.h` preservando ABI do `gdt.asm`:
//! - Layout `#[repr(C, packed)]` idêntico ao C `__attribute__((packed))`.
//! - Símbolo `i686_GDT_Initialize` com mesma assinatura `extern "C" fn()`.
//! - Chama `i686_GDT_Load(desc, 0x08, 0x10)` implementado em NASM
//!   (`lgdt` + far `retf` para recarregar CS + `mov ds/es/fs/gs/ss`).

use core::mem::size_of;

pub const CODE_SEGMENT: u16 = 0x08;
pub const DATA_SEGMENT: u16 = 0x10;

// --- Access byte (mesmos valores do enum GDT_ACCESS em gdt.c) ---
pub const ACCESS_PRESENT: u8 = 0x80;
pub const ACCESS_RING0: u8 = 0x00;
pub const ACCESS_CODE_SEGMENT: u8 = 0x18;
pub const ACCESS_DATA_SEGMENT: u8 = 0x10;
pub const ACCESS_CODE_READABLE: u8 = 0x02;
pub const ACCESS_DATA_WRITEABLE: u8 = 0x02;

// --- Flags + nibble alto do limite (mesmos valores de GDT_FLAGS) ---
pub const FLAG_32BIT: u8 = 0x40;
pub const FLAG_GRANULARITY_4K: u8 = 0x80;

/// Entrada de 8 bytes da GDT. Deve permanecer `packed` para o `lgdt`.
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

/// Descritor GDTR de 6 bytes (limit u16 + base u32).
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtDescriptor {
    limit: u16,
    base: u32,
}

const _: () = assert!(size_of::<GdtEntry>() == 8);
const _: () = assert!(size_of::<GdtDescriptor>() == 6);

impl GdtEntry {
    /// Equivalente `const` da macro `GDT_ENTRY(base, limit, access, flags)` do C.
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

// Tabela viva enquanto o kernel executa — por isso `static mut` + acesso em
// `init` com interrupções desabilitadas (o `_start` faz `cli` antes de chamar).
static mut GDT: [GdtEntry; 3] = [
    // Descritor NULL
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

extern "C" {
    fn i686_GDT_Load(desc: *const GdtDescriptor, code: u16, data: u16);
}

/// Ponto de entrada chamado pelo `boot/entry.asm`.
/// Mantém o nome/símbolo exato do C para não tocar o ASM.
#[no_mangle]
pub extern "C" fn i686_GDT_Initialize() {
    // SAFETY: `GDT` é estática e vive por todo o kernel; construímos o
    // descritor na stack (o `lgdt` copia limit+base para o GDTR, então o
    // temporário não precisa sobreviver). Chamado uma vez no boot com `cli`,
    // sem concorrência. A função ASM recarrega CS via far-ret e os segmentos
    // de dados — comportamento idêntico ao `gdt.c` original.
    unsafe {
        let desc = GdtDescriptor {
            limit: (size_of::<[GdtEntry; 3]>() - 1) as u16,
            base: core::ptr::addr_of!(GDT) as *const GdtEntry as u32,
        };
        i686_GDT_Load(&desc, CODE_SEGMENT, DATA_SEGMENT);
    }
}
