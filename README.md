# YoRunix - A Minimal Microkernel

YoRunix is a lightweight, educational microkernel written in x86 32-bit assembly and C. It focuses on demonstrating fundamental operating system concepts including bootloading, protected mode, process management, and interrupt handling.

## Overview

YoRunix is designed as a **microkernel architecture** where the core kernel remains small and minimal, with most operating system services implemented as user-space processes or modules. This approach promotes:

- **Modularity**: Core services are decoupled and can be developed independently
- **Stability**: A failure in a non-critical service doesn't crash the entire system
- **Extensibility**: New services can be added without modifying the kernel
- **Security**: Services run with minimal privileges, reducing attack surface

## Project Structure

```
yorunix/
├── boot/
│   └── boot.asm           # x86 bootloader and entry point (Multiboot compatible)
├── core/
│   └── kernel.c           # Microkernel core implementation
|-- divers/
|   |---vga.c
├── include/               # Header files
|   |---vga.h
├── Makefile              # Build configuration
└── link.ld               # Linker script

```

## Requirements

To build and run YoRunix, you need:

- **NASM** (Netwide Assembler) - for assembling x86 code
- **GCC** (with 32-bit support) - for compiling C code
- **GNU LD** (Linker) - for linking object files
- **QEMU** - for emulating x86 hardware

### Installation (Ubuntu/Debian)

```bash
sudo apt install nasm gcc-multilib g++-multilib binutils qemu-system-x86
```

## Building

Compile the kernel using the provided Makefile:

```bash
make
```

This will:
1. Assemble `boot/boot.asm` using NASM (32-bit ELF format)
2. Compile `core/*.c` using GCC with freestanding flags
3. Link all object files into `build/kernel.bin`

### Build Output

- `build/boot.o` - Compiled bootloader
- `build/kernel.o` - Compiled kernel core
- `build/kernel.bin` - Final executable kernel image

## Running

To execute the kernel in QEMU:

```bash
make run
```

This launches the kernel in a QEMU x86 emulator with 32-bit mode.

## Cleaning

Remove all build artifacts:

```bash
make clean
```

## Bootloader (boot/boot.asm)

The bootloader is **Multiboot-compliant**, allowing GRUB or other Multiboot loaders to boot the kernel. Key features:

- **Magic Number**: `0x1BADB002` - Identifies the kernel as Multiboot-compatible
- **Stack Setup**: Allocates 16 KB for kernel stack
- **Entry Point**: `_start` - Where execution begins after boot
- **Kernel Call**: Transfers control to `kernel_main()` function

The bootloader handles the transition from BIOS 16-bit real mode to 32-bit protected mode (handled by the bootloader, not by this code).

## Microkernel Architecture

YoRunix follows microkernel design principles:

### Core Kernel Responsibilities
- Interrupt/exception handling
- Process/thread scheduling
- Memory management (basic paging)
- Inter-process communication (IPC)

### User-Space Services (Future)
- File system
- Device drivers
- Network stack
- System utilities

This separation allows the kernel to remain small (&lt;100 KB) while delegating complex functionality to user-space.

## Development Roadmap

- [x] Bootloader setup (Multiboot)
- [x] Build system (Makefile)
- [x] GDT (Global Descriptor Table) setup
- [ ] IDT (Interrupt Descriptor Table) setup
- [ ] Basic process management
- [ ] Memory management (paging)
- [ ] Inter-process communication
- [ ] Simple device drivers
- [ ] Basic file system

## References

Inspired by and based on concepts from:
- **"Operating Systems: Design and Implementation"** (Andrew S. Tanenbaum & Albert S. Woodhall) - *"Sistemas Operacionais: Projeto e Implementação"*

Additional references:
- [OSDev Wiki](https://wiki.osdev.org/)
- [Multiboot Specification](https://www.gnu.org/software/grub/manual/multiboot/)
- [Intel x86 Architecture](https://en.wikipedia.org/wiki/X86)
- [Microkernel Architecture](https://en.wikipedia.org/wiki/Microkernel)

## License

YoRunix is provided for educational purposes. Feel free to modify and learn from it.

## Author

Vito - Educational OS Development Project
