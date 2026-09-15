# IDT — exceptions and interrupt stubs

**Source:** `src/idt.rs`, `arch/x86/idt.asm`

## How it works

### Two tables, one idea

The GDT answers *"which segments exist"*; the IDT answers *"what code runs when the CPU raises vector N"*. The CPU supports 256 vectors:

- **0–31**: CPU exceptions (divide error, page fault, …) — hardware-defined
- **32–47**: hardware IRQs (timer, keyboard…) — after PIC remapping *(future work)*
- **48–255**: software interrupts / syscalls

YoRunix currently wires the **first 32** — the exception gates — into `static mut IDT: [IdtEntry; 256]` (`src/idt.rs:62-68`) and loads the table with `lidt` (`src/idt.rs:184`).

### Anatomy of a gate (`src/idt.rs:19-27`)

| Field | Size | Contents |
|---|---|---|
| `base_low` | u16 | Handler address bits 0–15 |
| `selector` | u16 | Code segment of the handler (our `0x08`) |
| `zero` | u8 | Always 0 |
| `attr` | u8 | Type/privilege byte (below) |
| `base_high` | u16 | Handler address bits 16–31 |

Note the **`u16` on `base_high`**: this exact field is where the C original had a bug — see the migration log in [rust-for-osdev.md](rust-for-osdev.md). A gate must be **exactly 8 bytes**; the CPU reads entries at `IDT_base + 8*N` with no search or tolerance.

`attr = PRESENT | RING0 | TYPE_INTERRUPT_GATE` = `0x80 | 0x00 | 0x0E` = **`0x8E`** (`src/idt.rs:13-16, 164`). "Interrupt gate" means the CPU clears IF on entry (nested exceptions still possible via NMIs) — the standard choice for kernel exception handlers.

### Which vectors push an error code

On some exceptions the CPU pushes an extra **error code** dword (vectors 8, 10–14, 17; vector 30 on newer CPUs). The NASM side handles this with two macros (`arch/x86/idt.asm:9-24`):

```asm
ISR_NOERRCODE %1:   push byte 0     ; dummy — keep the frame layout uniform
                    push byte %1    ; vector number
ISR_ERRCODE   %1:   push byte %1    ; CPU already pushed the error code
```

Pushing a dummy for error-code vectors (or *not* pushing one for no-error vectors) would misalign the whole frame by 4 bytes — the classic first-kernel crash. Vector 21 (Control Protection) is deliberately kept in the NOERR group with a source comment; vector 30 uses the ERRCODE form — both per the code's annotations (`idt.asm:47, 56`).

### The interrupt stack frame

Everything combined, in the exact order the `InterruptFrame` struct mirrors (`src/idt.rs:41-57`):

```
higher addresses
+--------------+
|    eflags    |  ┐
|      cs      |  │  pushed by the CPU on every exception/interrupt
|      eip     |  ┘
|  error_code  |  ← CPU only on some vectors; stub pushes dummy 0 otherwise
|   int_num    |  ← pushed by every NASM stub
|     eax      |  ┐
|     ecx      |  │
|     edx      |  │
|     ebx      |  │  pusha — 8 registers
|   esp_save   |  │  (present in the image, skipped by popa)
|     ebp      |  │
|     esi      |  │
|     edi      |  ┘  ← ESP points here when the handler runs
+--------------+
lower addresses
```

The struct is `#[repr(C)]` and is **the contract between two languages**: if NASM's push order or the CPU's push order changes, this struct must change with it — otherwise the handler reads registers as other registers. (See the module docstring `src/idt.rs:1-9`.)

### The common handler trampoline (`arch/x86/idt.asm:59-75`)

```asm
i686_ISR_common:
    pusha                      ; save all general registers
    mov ax, 0x10               ; reload data segments to kernel data
    mov ds/es/fs/gs, ax
    push esp                   ; argument: pointer to the frame (cdecl)
    call i686_ISR_handler      ; → Rust (src/idt.rs:156)
    add esp, 4                 ; drop the argument
    popa                       ; restore registers
    add esp, 8                 ; drop int_num + error_code
    iret                       ; restore eip/cs/eflags (and error code) atomically
```

`iret` pops `eip`, `cs`, `eflags` — and the error code is consumed by it, which is why the cleanup `add esp, 8` only removes the two stub-pushed dwords.

### Registration (`src/idt.rs:162-186`)

`idt_init` — the symbol `_start` calls — fills gates 0..31 with `make_gate(handler as u32, 0x08, 0x8E)` via the `set_gate` helper, then executes `lidt` through inline assembly:

```rust
core::arch::asm!("lidt [{}]", in(reg) &desc, options(nostack, preserves_flags));
```

A single 3-byte instruction; no NASM round-trip needed. `make_gate` is a `const fn` that never touches the global table — encoding is pure and host-testable (`src/idt.rs:197-211`).

### The handler is deliberately a no-op

`i686_ISR_handler` currently does nothing with the frame (`src/idt.rs:155-159`), mirroring the C original's `(void)frame`. That's phase-one bring-up: before adding logic, the machinery must provably not crash — a fault occurring inside an exception handler is the worst class of bug to debug. Per-vector dispatch (print, panic with details, or ignore) is the next step on the [roadmap](../README.md#development-roadmap).

## Why we did this

- **Stub-in-ASM, logic-in-Rust.** The stack surgery (`pusha`, dummy pushes, segment reloads) must be cycle-exact assembly; everything above that line — the C-equivalent handler — is Rust. The seam is one `call` with a pointer argument.
- **Uniform frame layout.** Pushing a dummy error code for vectors 0–7/9/15–29 makes every exception arrive with an identical frame shape. One `InterruptFrame` type, one handler signature — no per-vector `if` on the C side.
- **cfg(test) stubs.** NASM objects don't participate in the host test link, so a macro generates no-op `i686_ISR0..31` symbols for tests (`src/idt.rs:111-130`) — letting the gate-encoding logic be unit-tested without an emulator. (See [rust-for-osdev.md](rust-for-osdev.md#testing-without-hardware).)

## Why it matters

Exceptions are **how the CPU talks to the operating system**: a page fault, a divide-by-zero, a GP fault — all arrive through this exact pipeline. Every driver interrupt and every syscall entry will reuse the machinery built here. The two universal lessons:

1. **Frame discipline.** Interrupt code is stack bookkeeping; one wrong `push`/`pop` and `iret` returns to a corrupted context. The `InterruptFrame` struct *is* the specification of that bookkeeping.
2. **Bring-up ordering.** An IDT full of no-op handlers is still progress — it converts "triple fault, QEMU restarts" into "exception happened, we're alive". Observable failure beats silent failure.

## Going deeper

- [Intel SDM Vol. 3, Chapter 6](https://www.intel.com/sdm) — exception and interrupt reference, error codes per vector
- [OSDev Wiki — IDT](https://wiki.osdev.org/IDT) — gate format and loading
- [OSDev Wiki — Interrupts](https://wiki.osdev.org/Interrupts) and [Exceptions](https://wiki.osdev.org/Exceptions) — vector map, error-code semantics
- [Writing an OS in Rust — Interrupts](https://os.phil-opp.com/interrupts/) — the modern long-mode version of this same machinery
