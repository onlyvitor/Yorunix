# YoRunix — Documentation

YoRunix is an educational 32-bit x86 microkernel: the kernel core is written in **Rust** (`no_std`, compiled as a static library), the low-level entry point and CPU-table plumbing in **NASM assembly**, booted through a **Multiboot**-compliant loader (GRUB or QEMU's built-in loader).

Every doc here answers three questions about its component:

1. **How it works** — the hardware or specification mechanics, anchored to the real code
2. **Why we did this** — the design decision, the alternative rejected, and the bugs this choice avoids
3. **Why it matters** — the OS-development concept you take away

Docs are written in a hybrid style: a concise reference body tied to the source, plus a *Going deeper* section with external material for the full specification details.

## Reading order

The docs form a narrative, best read in order:

| # | Doc | Source files | Learn about |
|---|-----|--------------|-------------|
| 1 | [Boot flow](boot.md) | `boot/entry.asm`, `link.ld`, `boot/grub/grub.cfg` | Multiboot header, protected mode, kernel entry sequence |
| 2 | [Build system](build-system.md) | `Makefile`, `Cargo.toml`, `src/support.rs` | Freestanding linking, static libraries, panic strategies |
| 3 | [GDT](gdt.md) | `src/gdt.rs`, `arch/x86/gdt.asm` | x86 segments, descriptor encoding, `lgdt` + far `retf` |
| 4 | [IDT & interrupts](idt-interrupts.md) | `src/idt.rs`, `arch/x86/idt.asm` | CPU exceptions, ISR stubs, interrupt stack frames |
| 5 | [VGA driver](vga-driver.md) | `src/vga.rs` | Memory-mapped I/O, volatile access, first driver |
| 6 | [Rust for OS dev](rust-for-osdev.md) | all of `src/` | Why Rust for a kernel, and the C→Rust migration log |

The reading order is intentional: boot and build explain *how the code becomes a running kernel*, GDT/IDT/VGA are the three components the kernel initializes, and the Rust doc closes the loop by explaining *why the core is written the way it is* — including every bug inherited from the C original.

## Conventions used across the docs

- Code references use `path:line` when precision matters and plain paths otherwise.
- `unsafe` blocks in the source carry `SAFETY:` comments explaining the invariant that makes them sound; docs reference these instead of repeating them.
- Symbol names such as `i686_GDT_Initialize`, `idt_init` and `kernel_main` are **preserved from the C codebase** the kernel was migrated from. The NASM assembly never needed to change during the migration — that ABI discipline is documented in [rust-for-osdev.md](rust-for-osdev.md).

## Status

Current feature status and the roadmap live in the [root README](../README.md#development-roadmap). The docs describe the code as it is today; roadmap items (interrupt dispatch, paging, processes, IPC) will get their own docs as they land.
