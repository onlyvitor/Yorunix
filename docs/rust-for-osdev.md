# Rust for OS development — what it actually buys you

**Source:** all of `src/`; **history:** the kernel core was originally C (`core/kernel.c`, `drivers/vga.c`, `arch/x86/*.c`) and was migrated module-by-module to Rust. The assembly was never touched.

## The short version

C gives you total control and trusts you completely. Rust gives you the same control and **checks the dangerous parts at compile time**. On bare metal this matters more than anywhere else: there is no allocator to catch a double free, no process boundary to contain a wild pointer, no supervisor to restart a crashed driver — a C kernel's memory bugs are *silent corruption or a triple fault*, and Rust's equivalent bugs are, in the safe subset, *compile errors*.

The rest of this doc is concrete: what we gave up with `std`, what the language checked for us, where `unsafe` remains — and the migration log listing the real bugs the C code carried.

## `no_std` / `no_main`: counting on yourself

```rust
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
```

`src/lib.rs:7-8`

Dropping `std` means rebuilding its floor. In a hosted program `std` silently provides: a panic handler with formatting and backtraces, memory allocation, `memcpy`-family intrinsics, thread-local storage, and a `main`-driven runtime. A freestanding kernel provides these itself:

| Normally free | Where YoRunix provides it |
|---|---|
| `main` / C runtime | We don't — ASM owns `_start`, Rust exports `extern "C"` fns (`src/lib.rs:36-44`) |
| Panic handler | `src/lib.rs:24-32` — a `hlt` loop |
| `memcpy`/`memmove`/`memset`/`memcmp`/`bcmp` | `src/support.rs:17-63` — byte-wise, no libc |
| `rust_eh_personality` (unwind glue in `core`) | `src/support.rs:14-15` — unreachable no-op under `panic=abort` |

`no_main` is the flip side: Rust generates no entry point at all. `boot/entry.asm` owns `_start`; Rust only publishes `kernel_main`, `i686_GDT_Initialize`, and `idt_init` for it to call. The kernel is a **library the boot stub links against** (see [build-system.md](build-system.md)).

## Ownership without a GC

Rust's core rule — every value has exactly one owner, borrows can't outlive it or mutate while shared — does the job that C assigns to programmer discipline and runtime checks that don't exist in a kernel:

- **No use-after-free**: values die with their owner; the compiler tracks lifetimes. In C, a dangling pointer compiles fine and detonates at runtime — in a kernel, possibly under an interrupt.
- **No data races**: shared mutable state across contexts must go through synchronization the compiler can see. There's no threading yet, but the language is already shaped for when there is.
- **No silent aliasing bugs**: the code can't hold two mutable handles to the same table by accident.

The remaining escape hatches are explicit. The GDT and IDT tables are `static mut` (`src/gdt.rs:65`, `src/idt.rs:62`) — a deliberate, documented exception (`SAFETY` comments at `gdt.rs:63-64, 99-103` and `idt.rs:145-147, 165-167`): hardware-required globals, initialized once at boot under `cli`, no concurrency yet. Rust doesn't forbid this; it makes the risk **a reviewed, marked decision** instead of the default.

## `unsafe` as a door, not a room

Rust's honest proposition for OS dev: **the unsafe surface shrinks from "the whole codebase" to a few dozen audited lines.**

In the C kernel, *every* line could corrupt memory. In the Rust kernel, `unsafe` appears only where hardware demands it:

- two raw MMIO loops in the VGA driver (`vga.rs:40-46, 57-90`)
- the GDTR/IDTR descriptor construction and the `lgdt`/`lidt` loads (`gdt.rs:104-110`, `idt.rs:168-185`)
- the `static mut` table accesses above

Each carries a `SAFETY:` comment stating the invariant that makes it sound. Everything else — string handling, descriptor arithmetic, gate encoding — is checked Rust. A reviewer audits **the doors, not the rooms**.

## FFI discipline with assembly

The kernel's two-language boundary is a calling convention and a struct layout, and Rust encodes both:

- **`#[repr(C)]` / `#[repr(C, packed)]`** — `GdtEntry`, `GdtDescriptor`, `IdtEntry`, `InterruptFrame` are byte-identical to the C structs the NASM code was written against (packed = `__attribute__((packed))`).
- **`const _: () = assert!(size_of == N)`** — `gdt.rs:46-47` and `idt.rs:59-60` turn "the gate is 8 bytes, the GDTR is 6" into *build-time* invariants. Break the layout and the compile fails — not QEMU, three layers deep.
- **Symbol parity** — `kernel_main`, `i686_GDT_Initialize`, `idt_init`, `i686_ISR_handler` keep their exact C-era names (`#[no_mangle]`), which is *why* the assembly never changed during the migration. ABI names are load-bearing documentation.
- **`extern "C"` fn types** — `i686_GDT_Load(desc, code, data)` is declared with its C signature (`gdt.rs:86`), so a wrong call is a type error.

## Compile-time work

C kernels lean on macros for table construction; Rust has real constant evaluation:

- `GdtEntry::new` and `make_gate` are **`const fn`** (`gdt.rs:51-61`, `idt.rs:134-142`) — the GDT table is fully built into the binary at compile time, replacing the C `GDT_ENTRY` macro with testable code.
- The same functions double as the **unit-test surface**: encoding logic is pure, so `cargo test` on the host verifies the exact bytes (`0x9A/0xCF`, gate split of `0x12345678`) with no hardware involved.

## Panics on bare metal

```rust
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { loop { hlt } }
```

(`src/lib.rs:24-32`) With `panic = "abort"` and no one to catch anything, a panic is a controlled halt: `hlt` keeps the CPU quiet and QEMU's window shows a frozen screen — *the kernel state at the crash preserved*. No unwinding machinery exists to pull in code paths, which is also why `support.rs` needs only an empty `rust_eh_personality` (`support.rs:10-15`).

## Testing without hardware

The test build flips `std` back on (`cfg_attr(not(test), ...)` — `lib.rs:7-8`) and stubs the ASM symbols, so pure logic runs as ordinary `cargo test` on the host:

- `i686_GDT_Load` gets a no-op stub with the same symbol (`gdt.rs:91-93`)
- `i686_ISR0..31` get macro-generated no-op stubs (`idt.rs:111-130`)
- the NASM objects simply don't link in test builds

What's tested: GDT entry encoding (including the `0x9A/0xCF` / `0x92/0xCF` flat-model values and base/limit field splitting), IDT gate encoding (`0x8E`, address split), null-entry zeroing. What isn't: anything touching real registers — that's what `make run` in QEMU is for.

One subtlety worth keeping: the `test` profile uses `panic = "unwind"` while dev/release use `abort` (`Cargo.toml:19-21`) because the libtest harness needs unwinding to *report* a failed assert; with abort, one failing test would abort the runner instead of logging a failure.

## Honest limits

- **`unsafe` didn't disappear** — MMIO, table loads, and global tables still require it, with `SAFETY` comments as the audit trail. Rust constrains the blast radius; it doesn't abolish hardware reality.
- **`static mut`** is a known debt: correct today (single-threaded boot, `cli`), and every SAFETY comment marks it for re-evaluation when concurrency arrives.
- **No allocator yet.** When one lands, Rust's ownership will matter even more — allocation bugs (leaks, double-frees, dangling) are precisely the class of bug the language eliminates in safe code.
- **Rust doesn't replace hardware knowledge.** You still must know what `lgdt` does, why gates are 8 bytes, and what `pusha` pushes. The language checks *your* understanding; it doesn't supply it.

## Migration log: bugs found in the C original

The most persuasive argument for the migration is that the C code carried real, observable bugs. Each row is documented in the module that fixed it:

| # | Bug in the C original | Symptom | Rust fix |
|---|---|---|---|
| 1 | `clear_screen` wrote `0x00` (NUL) and iterated `i < 80*25` with step 2 | Half the screen "cleared" with garbage glyphs | Blank cell `b' '`, full 2000-cell loop (`vga.rs:35-47`) |
| 2 | `putstr` ignored `\n`, wrap, and screen bounds | Writes past the visible screen into adjacent memory | Newline handling, wrap, bound checks (`vga.rs:53-91`) |
| 3 | MMIO stores without `volatile` | Optimizer legally allowed to elide "dead" stores | `write_volatile`/`read_volatile` on every access |
| 4 | IDT `base_high` was `uint8_t` → the gate struct was 7 bytes | Handler's top 8 address bits truncated and every gate after the first landed misaligned — corrupted dispatch | `base_high: u16`, 8-byte gate + `const` size assert (`idt.rs:26, 59`) |
| 5 | Multiboot header unprotected from `--gc-sections` | Header dropped or reordered after `.text` → GRUB: no multiboot header | Dedicated `.multiboot` section + `KEEP` in `link.ld:8-10` |
| 6 | Freestanding link lacked `memcpy`/`memset`/… | Undefined references when linking without a C driver | `support.rs` byte-wise libc-free implementations |

Notice the pattern: bugs 1–3 are *discipline* bugs C cannot see, bug 4 is a *layout* bug that compile-time size assertions would have caught instantly, bugs 5–6 are *toolchain* interactions that needed explicit ownership. The Rust migration didn't just translate the C — it made each failure mode structurally harder to reintroduce (bounds-checked writes, `const` asserts, `volatile` in the type system, tested encoding functions).

### The migration strategy (worth copying)

1. **Freeze the ABI, then swap internals.** All symbol names (`kernel_main`, `i686_GDT_Initialize`, `idt_init`, ISR symbols) were preserved with `#[no_mangle]`, so NASM assembly compiled once and never changed.
2. **One module at a time.** `vga.rs`, then `gdt.rs`, then `idt.rs` — each module docstring states which C file it replaces and what it fixes.
3. **Bring tests with each module.** Encoding logic became host-runnable the moment it became Rust; the tables' byte output is now pinned by `cargo test`.

## Going deeper

- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) — the unsafe contract, no_std, linking
- [Writing an OS in Rust (phil-opp)](https://os.phil-opp.com/) — the same journey at greater depth (long mode, testing on bare metal)
- [Embedded Rust Book](https://docs.rust-embedded.org/book/) — `no_std` patterns, volatile, peripheral drivers
- [Rust for Linux](https://rust-for-linux.github.io/) — Rust kernel modules in a production kernel, real-world FFI discipline
- [Undefined Behavior in C vs the `unsafe` contract](https://doc.rust-lang.org/nomicon/what-unsafe-does.html) — what `unsafe` permits (and what it still forbids)
