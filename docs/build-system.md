# Build system — from Rust and NASM to a bootable image

**Source:** `Makefile`, `Cargo.toml`, `rust-toolchain.toml`, `src/support.rs`

## How it works

### The pipeline

```
src/*.rs ──────── cargo build --target i686-unknown-linux-gnu ──▶ target/i686-unknown-linux-gnu/debug/libyorunix.a
                                                                      │
boot/entry.asm   ── nasm -f elf32 ──▶ build/entry.o                   │
arch/x86/gdt.asm ── nasm -f elf32 ──▶ build/gdt_asm.o                 │
arch/x86/idt.asm ── nasm -f elf32 ──▶ build/idt_asm.o                 │
                                                                      ▼
                  ld -m elf_i386 -z noexecstack --gc-sections -T link.ld
                                                                      │
                                                                      ▼
                                              build/kernel.bin   (Multiboot ELF32 image)
```

Three toolchains, one image:

1. **Cargo** compiles the kernel core (`src/`) into a **static library** — no boot logic, no entry point, just the Rust object code.
2. **NASM** assembles the three 32-bit stubs (Multiboot entry, GDT loader, IDT stubs).
3. **`ld`** (invoked directly) links everything into `build/kernel.bin` per `link.ld` — see [boot.md](boot.md) for the layout rules.

The `ifeq ($(ARCH), x86_64)` block (`Makefile:30-36`) keeps the door open for 64-bit codegen (`elf64` / `-m elf_x86_64`), though the current code is 32-bit throughout.

### Why a `staticlib` (`Cargo.toml:10`)

The crate is `crate-type = ["staticlib"]` with `#![no_main]` (`src/lib.rs:8`). Rust never generates a `main`, a C runtime, or an entry point — it produces an ordinary archive of object code. **Our assembly owns `_start`**, and Rust only exports `extern "C"` functions (`kernel_main`, `i686_GDT_Initialize`, `idt_init`, panic handler) for the assembly to call.

### The target (`rust-toolchain.toml`)

```toml
[toolchain]
channel = "stable"
targets = ["i686-unknown-linux-gnu"]
```

Rust needs a target spec to emit code; `i686-unknown-linux-gnu` gives 32-bit codegen whose object format and calling convention match NASM's `elf32` output and `ld`'s `elf_i386` mode. We use **only its codegen and ABI** — `no_std` means no Linux libraries are ever linked.

### Why link with `ld` directly — and what that costs

A normal Rust build lets `rustc` (which invokes a C compiler driver) drive the link. That would pull in `crt0`, `libc` and a `main` — none of which exist here. YoRunix links with bare `ld` instead, which has a **price the compiler forces us to pay**: LLVM emits references to C memory functions for anything non-trivial (struct copies → `memcpy`/`memmove`, `write_bytes` → `memset`, slice comparison → `memcmp`/`bcmp`), and the `core` crate references `rust_eh_personality` for its unwind tables even under `panic = "abort"`.

Nobody provides those symbols in a freestanding link — so we do, byte-wise and libc-free, in `src/support.rs:14-63`. This is not optional: without that file the link fails with undefined references (or worse, succeeds with the wrong semantic if a host libc sneaks in).

### Panic strategy (`Cargo.toml:12-17`)

| Profile | Setting | Why |
|---|---|---|
| `dev`, `release` | `panic = "abort"` | There is nothing to unwind to on bare metal. Unwind machinery would pull in landing pads and personality functions for no benefit. A panic is a controlled `hlt` loop (`src/lib.rs:26-32`). |
| `test` | *(unwind, forced by cargo)* | Cargo ignores explicit `panic` settings in the test profile and always uses unwinding — the libtest harness needs it to catch a failing `assert_eq!` and report it as a test failure instead of aborting the whole runner. This is why host tests work even though dev/release use `abort`. |

### Make targets

| Target | Does |
|---|---|
| `make` | Full build → `build/kernel.bin` |
| `make check` | `cargo check --target i686-unknown-linux-gnu` — fast type/borrow check, no link |
| `make run` | QEMU with `-kernel` (built-in Multiboot loader) |
| `make iso` | Builds a GRUB ISO via `grub-mkrescue` (needs `xorriso`) |
| `make run-grub` | Boots the ISO in QEMU through real GRUB |
| `make clean` | Removes `build/`, `iso/`, and Cargo `target/` |
| `make grub-install-instructions` | Prints per-distro install commands for the ISO tooling |

Unit tests run with plain `cargo test` on the host — see [rust-for-osdev.md](rust-for-osdev.md#testing-without-hardware) for how ASM symbols are stubbed out.

## Why we did this

- **Owning the final link.** A kernel is a program with no host below it; delegating link decisions to a driver built for hosted applications means fighting defaults. Owning `ld` invocation means owning the image layout.
- **Paying the freestanding tax explicitly.** `support.rs` exists *because* of the direct-`ld` choice. Writing it ourselves keeps the kernel honest: no hidden libc, fully auditable implementations.
- **Testing on the host.** Building a QEMU round-trip into every change would make iteration slow and failures opaque. The build splits logic that *can* be tested on the host (descriptor encodings — pure functions) from logic that can't (`lgdt`/`lidt` — only testable in an emulator).

## Why it matters

Linking is where freestanding projects die. The classic failure modes — undefined `memcpy`, a stray `main` symbol, the Multiboot header garbage-collected, unwind tables pulling in personality functions — are all **link-time** problems that no amount of correct C or Rust code prevents. This project deliberately walks through each one and documents the fix, so the next bare-metal build you do starts from knowledge, not from a 2 AM linker error.

## Going deeper

- [The Rustonomicon — Linking](https://doc.rust-lang.org/nomicon/linking.html) — `staticlib`, symbol requirements, panic strategies
- [The Rustonomicon — `no_std`](https://doc.rust-lang.org/nomicon/no_std.html) — what the standard library actually provides
- [Cargo Book — Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) — `panic`, `opt-level`, and profile inheritance
- [binutils `ld` manual](https://sourceware.org/binutils/docs/ld/) — `--gc-sections`, `-z noexecstack`, emulation modes
