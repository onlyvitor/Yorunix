# YoRunix - A Minimal Microkernel

[![CI](https://github.com/onlyvitor/Yorunix/actions/workflows/ci.yml/badge.svg)](https://github.com/onlyvitor/Yorunix/actions/workflows/ci.yml)

YoRunix is a lightweight, educational microkernel for 32-bit x86. The kernel core is written in **Rust** (`no_std`, `no_main`, compiled as a `staticlib`), with the low-level boot and CPU-table plumbing in **x86 32-bit assembly (NASM)**. It focuses on demonstrating fundamental operating system concepts: Multiboot booting, protected mode, descriptor tables (GDT/IDT), and bare-metal output on VGA text mode.

## Overview

YoRunix is designed as a **microkernel architecture** where the core kernel remains small and minimal, with most operating system services planned as user-space processes or modules. This approach promotes:

- **Modularity**: Core services are decoupled and can be developed independently
- **Stability**: A failure in a non-critical service doesn't crash the entire system
- **Extensibility**: New services can be added without modifying the kernel
- **Security**: Services run with minimal privileges, reducing attack surface

## Documentation

In-depth design docs live in [`docs/`](docs/README.md), each explaining *how it works, why we did it, and why it matters*:

- [Boot flow](docs/boot.md) — Multiboot, protected mode, the entry sequence
- [Build system](docs/build-system.md) — freestanding linking of the Rust staticlib and NASM objects
- [GDT](docs/gdt.md) / [IDT & interrupts](docs/idt-interrupts.md) — CPU tables and exception handling
- [VGA driver](docs/vga-driver.md) — the first driver and memory-mapped I/O
- [Rust for OS dev](docs/rust-for-osdev.md) — why the core is Rust, with the C→Rust migration log

## Project Structure

```
yorunix/
├── arch/
│   └── x86/
│       ├── boot/
│       │   └── entry.asm       # Multiboot header, stack setup and _start entry point
│       ├── asm/
│       │   ├── gdt.asm         # i686_GDT_Load: lgdt + segment reload (far retf)
│       │   └── idt.asm         # ISR stubs 0..31 + common handler trampoline
│       └── cpu/                # (Rust, em src/arch/x86/cpu) IdtEntry/IdtDescriptor/InterruptFrame
├── boot/
│   └── grub.cfg                # GRUB menu configuration for the bootable ISO
├── src/                        # Kernel core in Rust (no_std, staticlib)
│   ├── lib.rs                  # Crate root: kernel_main + panic handler (hlt loop)
│   ├── arch/
│   │   └── x86/
│   │       └── cpu/
│   │           ├── gdt.rs      # GDT construction (null, kernel code, kernel data)
│   │           └── idt.rs      # IDT gates + InterruptFrame + i686_ISR_handler dispatch
│   └── kernel/
│       ├── interrupts/
│       │   └── exceptions.rs   # Handlers semânticos (divide_error, debug, ...)
│       ├── drivers/
│       │   └── vga.rs          # VGA text-mode driver (80x25, MMIO 0xB8000)
│       └── support.rs          # Freestanding memcpy/memset/memcmp/bcmp + eh personality
├── Cargo.toml                  # Rust package (staticlib, panic = abort)
├── rust-toolchain.toml         # Stable toolchain pinned to i686-unknown-linux-gnu
├── Makefile                    # Build configuration
├── link.ld                     # Linker script (loads kernel at 1 MiB)
└── LICENSE                     # BSD 3-Clause
```

## Requirements

To build and run YoRunix, you need:

- **Rust** (stable, via `rustup`) - kernel core, targeting `i686-unknown-linux-gnu`
- **NASM** (Netwide Assembler) - for assembling the x86 32-bit stubs
- **GNU LD** (binutils) - for linking the final kernel image (`elf_i386`)
- **QEMU** - for emulating x86 hardware
- **grub-mkrescue + xorriso** *(optional)* - only for building the bootable ISO

### Installation

Install the Rust toolchain first (same on every distro), then the native tools for your distribution.

**Rust (any distro, via [rustup](https://rustup.rs)):**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add i686-unknown-linux-gnu
```

> The repository also pins the toolchain in `rust-toolchain.toml`: with rustup installed, entering the project directory resolves `stable` + the `i686-unknown-linux-gnu` target automatically.

**Ubuntu / Debian / Mint:**

```bash
sudo apt install nasm binutils qemu-system-x86

# Optional, only needed for `make iso` / `make run-grub`:
sudo apt install grub-common grub-pc-bin xorriso mtools
```

**Fedora:**

```bash
sudo dnf install nasm binutils qemu-system-x86

# Optional, only needed for `make iso` / `make run-grub`:
sudo dnf install grub2-tools grub2-pc-modules xorriso mtools
```

**Arch Linux:**

```bash
sudo pacman -S --needed nasm binutils qemu-desktop

# Optional, only needed for `make iso` / `make run-grub`:
sudo pacman -S --needed grub xorriso mtools
```

**openSUSE:**

```bash
sudo zypper install nasm binutils qemu-x86

# Optional, only needed for `make iso` / `make run-grub`:
sudo zypper install grub2 grub2-i386-pc xorriso mtools
```

> **Note (Fedora / openSUSE):** these distros name the GRUB tools `grub2-*` (e.g. `grub2-mkrescue`), while the Makefile calls `grub-mkrescue`. If `make iso` reports it missing, add a symlink:
>
> ```bash
> sudo ln -s "$(which grub2-mkrescue)" /usr/local/bin/grub-mkrescue
> ```

**Verify the toolchain:**

```bash
cargo --version && nasm -v && ld -v && qemu-system-x86_64 --version
```

## Building

Compile the kernel using the provided Makefile:

```bash
make
```

This will:
1. Compile the Rust kernel core with `cargo build --target i686-unknown-linux-gnu` into a `staticlib` (`libyorunix.a`)
2. Assemble `arch/x86/boot/entry.asm`, `arch/x86/asm/gdt.asm` and `arch/x86/asm/idt.asm` using NASM (32-bit ELF format)
3. Link everything with `ld -m elf_i386 -T link.ld` into `build/kernel.bin`

### Build Output

- `target/i686-unknown-linux-gnu/debug/libyorunix.a` - Rust kernel core (static library)
- `build/entry.o`, `build/gdt_asm.o`, `build/idt_asm.o` - Assembled NASM objects
- `build/kernel.bin` - Final executable Multiboot kernel image

### Checking and Testing

```bash
make check      # cargo check for the i686 target
cargo test      # host unit tests (GDT/IDT gate encoding, etc.)
```

Unit tests run on the host: the `std` panic handler is used there, and NASM symbols are replaced by cfg-gated no-op stubs, so the tests validate pure logic (descriptor encodings, field splitting) without hardware.

## Running

To execute the kernel in QEMU using QEMU's built-in Multiboot loader:

```bash
make run
```

Or boot through a real GRUB flow by building an ISO (requires `grub-mkrescue` and `xorriso`):

```bash
make iso         # creates build/kernel.iso with boot/grub.cfg
make run-grub    # boots the ISO with QEMU (-cdrom, GRUB menu)
```

## Cleaning

Remove all build artifacts (Makefile outputs, ISO tree and Cargo `target/`):

```bash
make clean
```

## CI/CD

The pipeline runs on GitHub Actions (`.github/workflows/`):

- **CI** (`ci.yml`) — on every push to `main` and every pull request, two parallel jobs:
  - **Lint and test**: `cargo fmt --check`, `cargo clippy -D warnings` for both the host (covers the `cfg(test)` code) and the `i686-unknown-linux-gnu` target (the kernel's real codegen), plus the host unit tests
  - **Build and boot smoke test**: full `make` (cargo staticlib + NASM + `ld`), then a headless QEMU run — the kernel must survive 10 s in its `hlt` loop without any triple fault/reboot (verified by QEMU's `cpu_reset` log); `kernel.bin` is uploaded as a workflow artifact
- **Release** (`release.yml`) — pushing a tag (`git tag v0.1.0 && git push --tags`) runs the tests, builds the kernel and the GRUB ISO, and publishes both on a GitHub Release
- **Dependabot** (`dependabot.yml`) — keeps the workflow actions updated weekly

Recommended repository setting: enable branch protection on `main` and require the **Lint and test** and **Build kernel and boot smoke test** checks before merging.

## Boot Flow (arch/x86/boot/entry.asm)

The entry stub is **Multiboot-compliant**, allowing GRUB (or QEMU's `-kernel`) to boot the kernel. On boot:

1. **Multiboot header**: magic `0x1BADB002`, placed in a dedicated `.multiboot` section kept in the first 8 KiB of the image by the linker script
2. **Stack setup**: `cli`, then 16 KB stack allocated in `.bss`, 16-byte aligned
3. **GDT initialization**: calls `i686_GDT_Initialize` (Rust) → `i686_GDT_Load` (NASM: `lgdt`, far `retf` to reload CS, reload DS/ES/FS/GS/SS)
4. **IDT initialization**: calls `idt_init` (Rust) → fills 256 gates (`lidt`) with 32 exception stubs
5. **Kernel call**: transfers control to `kernel_main()` (Rust), which clears the VGA screen, prints a welcome message and `hlt`-loops

The transition from BIOS 16-bit real mode to 32-bit protected mode is performed by the bootloader (GRUB), not by this code.

## Kernel Components

### VGA text mode (src/kernel/drivers/vga.rs)
Safe wrapper over the memory-mapped text buffer at `0xB8000` (80x25 cells, white-on-black). Uses `write_volatile`/`read_volatile` so MMIO writes are never optimized away. `clear_screen` fills all 2000 cells with spaces; `putstr` handles `\n`, line wrap and screen-bounds truncation; `put_hex_at` writes positional `0xXXXXXXXX` values via the pure, host-tested `format_hex` helper. Driver and `support.rs` alike use only `while` byte loops — never helpers that lower to `memcpy`/`memmove`/`memset` (see `docs/vga-driver.md`).

### GDT (src/arch/x86/cpu/gdt.rs)
Three descriptors: NULL, kernel code (`0x08`) and kernel data (`0x10`), both ring 0, 32-bit, 4 KiB granularity (full 4 GB flat model). Entries are `#[repr(C, packed)]` with compile-time size assertions, keeping ABI compatibility with the NASM loader.

### IDT (src/arch/x86/cpu/idt.rs)
A 256-entry Interrupt Descriptor Table with the first 32 gates (CPU exceptions) wired to NASM stubs (`i686_ISR0`..`i686_ISR31`). Stubs push an interrupt number (plus a dummy 0 when the CPU doesn't push an error code), then trampoline into the common handler, which saves all registers (`pusha`), reloads data segments and calls the Rust `i686_ISR_handler` with an `InterruptFrame`. Vectors 0 (`#DE`) and 1 (`#DB`) dispatch to handlers in `kernel/interrupts/exceptions.rs` with an `EIP/CS/EFLAGS` VGA dump; the remaining vectors are still no-op placeholders.

### Freestanding support (src/kernel/support.rs)
Bare-metal linking without libc requires `memcpy`, `memmove`, `memset`, `memcmp`, `bcmp` and `rust_eh_personality`; this module provides minimal implementations with raw-pointer byte loops only — slice helpers and `ptr` intrinsics are banned here because they can lower back into the same symbols.

### Panic behavior
In the kernel (`panic = "abort"`, `no_std`), a panic enters a safe `hlt` loop - there is nothing to unwind into.

## Microkernel Architecture

YoRunix follows microkernel design principles:

### Core Kernel Responsibilities (current/planned)
- Boot and CPU tables: GDT, IDT, exception stubs *(done)*
- Interrupt/exception handling *(vectors 0–1 dispatch live, rest pending)*
- Memory management (basic paging)
- Process/thread scheduling
- Inter-process communication (IPC)

### User-Space Services (Future)
- File system
- Device drivers
- Network stack
- System utilities

This separation allows the kernel to remain small while delegating complex functionality to user-space.

## Development Roadmap

- [x] Build system (Makefile + Cargo staticlib)
- [x] Bootloader setup (Multiboot, GRUB ISO)
- [x] GDT (Global Descriptor Table) setup
- [x] IDT (Interrupt Descriptor Table) + exception stubs
- [x] VGA text-mode output driver
- [x] Partial interrupt dispatch in Rust (vectors 0–1 live, rest no-op)
- [ ] Full per-vector dispatch (`i686_ISR_handler` for all 32 vectors)
- [ ] IRQs: PIC remapping, timer and keyboard drivers
- [ ] Memory management (paging)
- [ ] Basic process management
- [ ] Inter-process communication
- [ ] Basic file system

## References

Inspired by and based on concepts from:
- **"Operating Systems: Design and Implementation"** (Andrew S. Tanenbaum & Albert S. Woodhull) - *"Sistemas Operacionais: Projeto e Implementação"*

Additional references:
- [OSDev Wiki](https://wiki.osdev.org/)
- [Multiboot Specification](https://www.gnu.org/software/grub/manual/multiboot/)
- [Rustonomicon - Freestanding Rust](https://doc.rust-lang.org/nomicon/what-does-unsafe-mean.html) & [Bare Metal Rust](https://os.phil-opp.com/)
- [Intel x86 Architecture](https://en.wikipedia.org/wiki/X86)
- [Microkernel Architecture](https://en.wikipedia.org/wiki/Microkernel)

## License

YoRunix is licensed under the BSD 3-Clause License - see [LICENSE](LICENSE).

## Author

Vitor (Vitor Gabriel Nascimento França) - Educational OS Development Project
