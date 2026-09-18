# IDT — exceptions and interrupt stubs

**Source:** `src/idt.rs`, `src/handler/idt_handler.rs`, `src/vga.rs`, `arch/x86/idt.asm`

## How it works

### Two tables, one idea

The GDT answers *"which segments exist"*; the IDT answers *"what code runs when the CPU raises vector N"*. The CPU supports 256 vectors:

- **0–31**: CPU exceptions (divide error, page fault, …) — hardware-defined
- **32–47**: hardware IRQs (timer, keyboard…) — after PIC remapping *(future work)*
- **48–255**: software interrupts / syscalls

YoRunix currently wires the **first 32** — the exception gates — into `static mut IDT: [IdtEntry; 256]` (`src/idt.rs:62-68`) and loads the table with `lidt` (`src/idt.rs:210`).

### Anatomy of a gate (`src/idt.rs:19-27`)

| Field | Size | Contents |
|---|---|---|
| `base_low` | u16 | Handler address bits 0–15 |
| `selector` | u16 | Code segment of the handler (our `0x08`) |
| `zero` | u8 | Always 0 |
| `attr` | u8 | Type/privilege byte (below) |
| `base_high` | u16 | Handler address bits 16–31 |

Note the **`u16` on `base_high`**: this exact field is where the C original had a bug — see the migration log in [rust-for-osdev.md](rust-for-osdev.md). A gate must be **exactly 8 bytes**; the CPU reads entries at `IDT_base + 8*N` with no search or tolerance.

`attr = PRESENT | RING0 | TYPE_INTERRUPT_GATE` = `0x80 | 0x00 | 0x0E` = **`0x8E`** (`src/idt.rs:13-16, 185`). "Interrupt gate" means the CPU clears IF on entry (nested exceptions still possible via NMIs) — the standard choice for kernel exception handlers.

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
    call i686_ISR_handler      ; → Rust (src/idt.rs:165)
    add esp, 4                 ; drop the argument
    popa                       ; restore registers
    add esp, 8                 ; drop int_num + error_code
    iret                       ; restore eip/cs/eflags (and error code) atomically
```

`iret` pops `eip`, `cs`, `eflags` — and the error code is consumed by it, which is why the cleanup `add esp, 8` only removes the two stub-pushed dwords.

### Registration (`src/idt.rs:184-212`)

`idt_init` — the symbol `_start` calls — fills gates 0..31 with `make_gate(handler as u32, 0x08, 0x8E)` via the `set_gate` helper, then executes `lidt` through inline assembly:

```rust
core::arch::asm!("lidt [{}]", in(reg) &desc, options(nostack, preserves_flags));
```

A single 3-byte instruction; no NASM round-trip needed. `make_gate` is a `const fn` that never touches the global table — encoding is pure and host-testable (`src/idt.rs:225-236`).

### Dispatch: vectors 0–1 are live (`src/idt.rs:151-180`)

`i686_ISR_handler` is no longer a no-op. It null-guards the ASM-passed `*mut InterruptFrame`, then dispatches on `int_num` via named constants (`DIVIDE_VECTOR = 0`, `DEBUG_VECTOR = 1`), both pinned by host tests:

| Vector | Handler (`src/handler/idt_handler.rs`) | Class | Behavior |
|---|---|---|---|
| 0 `#DE` | `divide_error:9` | Fatal fault, no error code | Dumps `EIP/CS/EFLAGS`, then `cli/hlt` loops forever — `iret` would re-execute the same faulting `div` |
| 1 `#DB` | `debug:35` | Recoverable fault, no error code | Dumps `EIP/CS/EFLAGS`, then **returns** so `iret` resumes (single-step / hw breakpoint) |

Vectors 2–31 still fall through to no-op, preserving prior behavior until each gets its handler.

Both dumps share one screen layout (positional model, no global cursor — see [VGA driver](vga-driver.md)): one `putstr` prints the labels (`Title\nEIP=\nCS=\nEFLAGS=` on rows 0–3), then `put_hex_at(v, row, col)` (`src/vga.rs:126`) writes each `0xXXXXXXXX` value at the end of its label (`(1,4)`, `(2,3)`, `(3,7)`). Fields are copied to locals before the volatile MMIO writes.

FFI note: the handler keeps its `extern "C"` ABI for the NASM `call`, so Clippy's `not_unsafe_ptr_arg_deref` is suppressed locally with a `SAFETY` justification instead of marking it `unsafe fn`.

## Why we did this

- **Stub-in-ASM, logic-in-Rust.** The stack surgery (`pusha`, dummy pushes, segment reloads) must be cycle-exact assembly; everything above that line — the C-equivalent handler — is Rust. The seam is one `call` with a pointer argument.
- **Uniform frame layout.** Pushing a dummy error code for vectors 0–7/9/15–29 makes every exception arrive with an identical frame shape. One `InterruptFrame` type, one handler signature — no per-vector `if` on the C side.
- **cfg(test) stubs.** NASM objects don't participate in the host test link, so a macro generates no-op `i686_ISR0..31` symbols for tests (`src/idt.rs:111-130`) — letting the gate-encoding logic be unit-tested without an emulator. (See [rust-for-osdev.md](rust-for-osdev.md#testing-without-hardware).)

## Why it matters

Exceptions are **how the CPU talks to the operating system**: a page fault, a divide-by-zero, a GP fault — all arrive through this exact pipeline. Every driver interrupt and every syscall entry will reuse the machinery built here. The two universal lessons:

1. **Frame discipline.** Interrupt code is stack bookkeeping; one wrong `push`/`pop` and `iret` returns to a corrupted context. The `InterruptFrame` struct *is* the specification of that bookkeeping.
2. **Bring-up ordering.** Vectors 0–1 print diagnostics while 2–31 stay no-op — it converts "triple fault, QEMU restarts" into "exception happened, we're alive". Observable failure beats silent failure.

## Going deeper

- [Intel SDM Vol. 3, Chapter 6](https://www.intel.com/sdm) — exception and interrupt reference, error codes per vector
- [OSDev Wiki — IDT](https://wiki.osdev.org/IDT) — gate format and loading
- [OSDev Wiki — Interrupts](https://wiki.osdev.org/Interrupts) and [Exceptions](https://wiki.osdev.org/Exceptions) — vector map, error-code semantics
- [Writing an OS in Rust — Interrupts](https://os.phil-opp.com/interrupts/) — the modern long-mode version of this same machinery
