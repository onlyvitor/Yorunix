# Boot flow — from power-on to `kernel_main`

**Source:** `arch/x86/boot/entry.asm`, `link.ld`, `boot/grub.cfg`

## How it works

### The chain

```
 BIOS (16-bit real mode)
   │  loads the boot sector from disk / CD, runs the bootloader
   ▼
 GRUB (Multiboot loader)
   │  scans kernel.bin for the Multiboot header (must be in the first 8 KiB)
   │  switches the CPU to 32-bit protected mode
   │  loads the kernel at 1 MiB and jumps to _start
   ▼
 _start (arch/x86/boot/entry.asm)
   │  cli → set up 16 KB stack → i686_GDT_Initialize → idt_init
   ▼
 kernel_main (src/lib.rs)
      clears the screen, prints the welcome string, hlt loop
```

The transition from BIOS real mode to protected mode is **GRUB's job**, not ours. YoRunix deliberately starts life in 32-bit protected mode with a flat memory view.

### The Multiboot header (`arch/x86/boot/entry.asm:1-9`)

```asm
section .multiboot
align 4
MBOOT_MAGIC    equ 0x1BADB002
MBOOT_FLAGS    equ 0x0
MBOOT_CHECKSUM equ -(MBOOT_MAGIC + MBOOT_FLAGS)

dd MBOOT_MAGIC
dd MBOOT_FLAGS
dd MBOOT_CHECKSUM
```

- **Magic** `0x1BADB002` — the constant every Multiboot loader searches for.
- **Flags** `0x0` — the "no requirements" flag set: the loader owes us nothing extra (no memory map, no video mode info, modules not page-aligned).
- **Checksum** — defined so `magic + flags + checksum ≡ 0 (mod 2³²)`. GRUB validates this sum; get it wrong and the header is ignored.
- **`align 4`** — the Multiboot spec requires the header to be 32-bit aligned.
- **Its own section**, `.multiboot`, exists *only* to control placement — see below.

### The linker script's role (`link.ld`)

```ld
. = 1M;                                  /* load address: 1 MiB */
.multiboot : { KEEP(*(.multiboot)) }     /* first thing in the image */
.text : { *(.text*) }
...
```

Two decisions here:

1. **Load at 1 MiB.** Convention dating back to the IBM PC: addresses below 1 MiB are littered with legacy BIOS structures (IVT, VGA buffers, BIOS data area). Above 1 MiB is clean territory.
2. **`KEEP(*(.multiboot))` in a dedicated output section.** The Makefile links with `--gc-sections` (`Makefile:27`) — garbage collection for unused input sections. Without a dedicated output section + `KEEP`, the linker is free to drop the header (it's pure data, nothing references it) or shuffle it after the Rust `.text` — both break booting: the spec requires the magic within the **first 8 KiB of the image**. GRUB would report `no multiboot header found`. This is a real bug class, not a theoretical one — the C-era linker script hit it.

### What `_start` does (`arch/x86/boot/entry.asm:24-33`)

```asm
_start:
    cli                        ; interrupts off — no IDT exists yet
    mov esp, stack_top         ; 16 KB stack from .bss (align 16)
    call i686_GDT_Initialize   ; Rust: build table, NASM: lgdt + reload segments
    call idt_init              ; Rust: build table, lidt
    call kernel_main           ; Rust: never returns
.hang:
    hlt
    jmp .hang                  ; fallback only — kernel_main never returns
```

- **`cli` first.** Until an IDT exists, any interrupt (timer tick, NMI aside) would vector into garbage and triple-fault the CPU. Interrupts stay off for the entire boot sequence in the current stage.
- **16 KB stack, 16-byte aligned** (`arch/x86/boot/entry.asm:11-16`). The x86 ABI assumes stack alignment for SSE instructions (`movaps` faults on a misaligned access), and 16 KB is comfortable room for a kernel that will grow.
- **`hlt` loop as fallback.** `kernel_main` is typed `-> !` (never returns) in Rust, so the `.hang` label is defense in depth: if control ever did return, the CPU parks instead of executing whatever follows.

### Two ways to boot it

| Command | Loader | Notes |
|---|---|---|
| `make run` | QEMU's **built-in Multiboot loader** | No GRUB involved; `-kernel build/kernel.bin` (`Makefile:67-68`). Fast inner loop for development. |
| `make iso` + `make run-grub` | **GRUB** from a CD image | `grub-mkrescue` packages `kernel.bin` + `boot/grub.cfg` (`multiboot /boot/kernel.bin`) into `build/kernel.iso`. Closer to real hardware. |

Same kernel image, two loaders — a consequence of Multiboot compliance that pays for itself during development.

## Why we did this

- **Lean on GRUB instead of writing a bootloader.** The educational payload of YoRunix is the kernel, not disk geometry and FAT parsing. Multiboot gives us a standards-compliant entry into protected mode for free — and QEMU's `-kernel` support gives a sub-second rebuild-boot loop.
- **A hand-written linker script instead of defaults.** A freestanding kernel cannot trust default linker behavior: we need a fixed load address, a guaranteed header placement, and no C runtime scaffolding.
- **`cli` before touching anything.** Boot-time discipline: the machine is in an inconsistent state until *we* make it consistent, and only interrupts can surprise us mid-way.

## Why it matters

Every "hello world" kernel tutorial shows you a Multiboot header, but the *placement* rules are where real projects die: a header that works in one build can silently fail in the next after enabling `--gc-sections`. Understanding that the header is **data discovered by scanning bytes** — not a function that gets called — explains a whole family of boot failures. The same discipline (explicit sections, `KEEP`, load address) carries over to every embedded and kernel project you will ever link.

## Going deeper

- [Multiboot 0.6.96 specification](https://www.gnu.org/software/grub/manual/multiboot/multiboot.html) — the header spec, checksum math, and machine state at entry
- [OSDev Wiki — Boot Sequence](https://wiki.osdev.org/Boot_Sequence) — BIOS to kernel, real mode to protected mode
- [OSDev Wiki — GRUB](https://wiki.osdev.org/GRUB) and [QEMU options](https://wiki.osdev.org/QEMU) — booting workflows
- [Linker Scripts (ld manual)](https://sourceware.org/binutils/docs/ld/Scripts.html) — `KEEP`, `--gc-sections`, section control
