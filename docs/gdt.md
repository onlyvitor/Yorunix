# GDT — the Global Descriptor Table

**Source:** `src/gdt.rs`, `arch/x86/gdt.asm`

## How it works

### Why segments exist (even in "flat" mode)

In x86 protected mode, **every memory access goes through a segment descriptor** — even in a "flat" model where all segments cover the whole 4 GiB. The CS, DS, ES, FS, GS, SS registers don't hold addresses; they hold *selectors*, indices into the GDT (or LDT). When the CPU starts in protected mode, whatever segments the bootloader loaded point into **its** tables — which die the moment we overwrite them. A kernel must install its own GDT before doing anything memory-related.

### Anatomy of an entry (`src/gdt.rs:27-36`)

An entry is exactly 8 bytes:

| Field | Size | Contents |
|---|---|---|
| `limit_low` | u16 | Limit bits 0–15 |
| `base_low` | u16 | Base bits 0–15 |
| `base_middle` | u8 | Base bits 16–23 |
| `access` | u8 | Permissions/type byte (below) |
| `flags_limit_hi` | u8 | Limit bits 16–19 (low nibble) + flags (high nibble) |
| `base_high` | u8 | Base bits 24–31 |

The base/limit split across five fields is 80286-era legacy — it's why `#[repr(C, packed)]` is mandatory: the CPU reads raw bytes at fixed offsets, and any padding would corrupt the encoding.

### The access byte

| Bit | Name | Meaning |
|---|---|---|
| 7 | Present | Segment is valid (always 1 for us) |
| 6–5 | DPL | Descriptor Privilege Level (0 = kernel) |
| 4 | Type | 1 = code/data, 0 = system |
| 3 | Executable | 1 = code segment |
| 2 | Direction/Conforming | growth direction / conformance |
| 1 | Readable/Writable | code: readable, data: writable |

Our two real entries (`src/gdt.rs:69-82`):

- **Kernel code**: `0x80 | 0x00 | 0x18 | 0x02` = **`0x9A`** (present, ring 0, code, readable)
- **Kernel data**: `0x80 | 0x10 | 0x02` = **`0x92`** (present, ring 0, data, writable)

### Flags and the 4 GiB flat model

`flags_limit_hi = 0xCF` = limit-high `0xF` + flags `0xC0`:

- **Granularity (4 KiB)**: the limit counts 4 KiB pages, not bytes.
- **32-bit**: operands and addresses are 32-bit.

So limit `0xFFFFF` pages × 4 KiB = **4 GiB**, base `0x00000000` — the flat model. Both entries are unit-tested against exactly these bytes (`src/gdt.rs:130-157`: `0x9A/0xCF` and `0x92/0xCF`).

### Selectors

Selectors are `(index << 3) | RPL`. With the NULL descriptor at index 0:

- **`0x08`** — index 1 = kernel code
- **`0x10`** — index 2 = kernel data

These constants (`src/gdt.rs:11-12`) are reused by the IDT (`SELECTOR_KERNEL_CODE = 0x08`) — the tables agree on who's who.

### The load sequence (`src/gdt.rs:97-111` + `arch/x86/gdt.asm`)

```
i686_GDT_Initialize (Rust)
  │  builds GdtDescriptor { limit: size-1, base: &GDT } on the stack
  │  (lgdt copies limit+base into the GDTR — the temporary need not outlive the call)
  ▼
i686_GDT_Load (NASM, arch/x86/gdt.asm:5-34)
  │  lgdt [eax]                    ; load GDTR
  │  push <new CS>; push .reload_cs; retf   ; far-return to reload CS
  │  mov ds/es/fs/gs/ss, <data>    ; reload data segments
  ▼
back in _start, next line
```

The **far `retf` trick** is the interesting part: `CS` cannot be reloaded with a plain `mov` (loading a selector into CS would be a jump to somewhere undefined). The canonical solution pushes the new selector and a return address, then executes `retf` — which loads **both** CS:EIP from the stack, landing in the next instruction *with* the new code segment active.

### ABI discipline

- `#[repr(C, packed)]` mirrors the C `__attribute__((packed))` the NASM stub expects — the Rust side is a drop-in replacement for `gdt.c` (see [rust-for-osdev.md](rust-for-osdev.md)).
- `const _: () = assert!(size_of::<GdtEntry>() == 8)` (`src/gdt.rs:46-47`) — if anyone breaks the layout, **the build fails at compile time**, not in QEMU three layers deep.
- `GdtEntry::new` is a `const fn` (`src/gdt.rs:51-61`): descriptors are built at compile time into the `static GDT` table — zero runtime init cost, same job the C macro `GDT_ENTRY` did.

### Why `static mut` is acceptable here

`GDT` is `static mut` — a global the borrow checker can't guard. The justification lives in the SAFETY comment (`src/gdt.rs:63-64, 99-103`): the table must live for the entire kernel lifetime (the GDTR points into it forever), it's initialized exactly once during boot **under `cli`** from `_start`, and there is no concurrency yet. When SMP or preemption arrives, this invariant needs revisiting — the comment marks where.

## Why we did this

- **Preserve the C ABI.** The NASM `i686_GDT_Load` and the symbol `i686_GDT_Initialize` were frozen so the C→Rust migration swapped `gdt.c` for `gdt.rs` **without touching a line of assembly**. Incremental, reviewable migration beats big-bang rewrites.
- **Encoding logic as pure functions.** `GdtEntry::new` and `make_gate`-style construction never touch the live table, so descriptor encoding is testable on the host (`cargo test`) without any hardware.

## Why it matters

Segments are the oldest protection machinery in x86, and modern kernels mostly route around them with paging — but you **cannot skip the GDT**: CS/SS must point at valid descriptors or the CPU faults on the first far operation. Understanding descriptor bytes also unlocks TSS setup (needed for hardware task switching and privilege transitions) later. The far `retf` reload is a pattern you'll meet again with `iret`, `ljmp`, and exception returns.

## Going deeper

- [Intel SDM Vol. 3, Chapter 3](https://www.intel.com/sdm) — protected-mode segmentation, descriptor formats
- [OSDev Wiki — GDT tutorial](https://wiki.osdev.org/GDT_Tutorial) — byte-by-byte descriptor construction
- [OSDev Wiki — Segment Selector](https://wiki.osdev.org/Segment_Selector) — selector encoding and RPL
- [Writing an OS in Rust — segmentation](https://os.phil-opp.com/) — GDT from Rust, later switched to long mode
