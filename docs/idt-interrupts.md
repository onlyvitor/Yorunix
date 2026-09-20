# IDT — exceptions and interrupt stubs

**Source:** `src/arch/x86/cpu/idt.rs`, `src/kernel/interrupts/exceptions.rs`, `src/kernel/drivers/vga.rs`, `arch/x86/asm/idt.asm`

## How it works

### Two tables, one idea

The GDT answers *"which segments exist"*; the IDT answers *"what code runs when the CPU raises vector N"*. The CPU supports 256 vectors:

- **0–31**: CPU exceptions (divide error, page fault, …) — hardware-defined
- **32–47**: hardware IRQs (timer, keyboard…) — after PIC remapping *(future work)*
- **48–255**: software interrupts / syscalls

YoRunix currently wires the **first 32** — the exception gates — into `static mut IDT: [IdtEntry; 256]` (`src/arch/x86/cpu/idt.rs:62-68`) and loads the table with `lidt` (`src/arch/x86/cpu/idt.rs:210`).

### Anatomy of a gate (`src/arch/x86/cpu/idt.rs:19-27`)

| Field | Size | Contents |
|---|---|---|
| `base_low` | u16 | Handler address bits 0–15 |
| `selector` | u16 | Code segment of the handler (our `0x08`) |
| `zero` | u8 | Always 0 |
| `attr` | u8 | Type/privilege byte (below) |
| `base_high` | u16 | Handler address bits 16–31 |

Note the **`u16` on `base_high`**: this exact field is where the C original had a bug — see the migration log in [rust-for-osdev.md](rust-for-osdev.md). A gate must be **exactly 8 bytes**; the CPU reads entries at `IDT_base + 8*N` with no search or tolerance.

`attr = PRESENT | RING0 | TYPE_INTERRUPT_GATE` = `0x80 | 0x00 | 0x0E` = **`0x8E`** (`src/arch/x86/cpu/idt.rs:13-16, 185`). "Interrupt gate" means the CPU clears IF on entry (nested exceptions still possible via NMIs) — the standard choice for kernel exception handlers.

### Which vectors push an error code

On some exceptions the CPU pushes an extra **error code** dword (vectors 8, 10–14, 17; vector 30 on newer CPUs). The NASM side handles this with two macros (`arch/x86/asm/idt.asm:9-24`):

```asm
ISR_NOERRCODE %1:   push byte 0     ; dummy — keep the frame layout uniform
                    push byte %1    ; vector number
ISR_ERRCODE   %1:   push byte %1    ; CPU already pushed the error code
```

Pushing a dummy for error-code vectors (or *not* pushing one for no-error vectors) would misalign the whole frame by 4 bytes — the classic first-kernel crash. Vector 21 (Control Protection) is deliberately kept in the NOERR group with a source comment; vector 30 uses the ERRCODE form — both per the code's annotations (`idt.asm:47, 56`).

### The interrupt stack frame

Everything combined, in the exact order the `InterruptFrame` struct mirrors (`src/arch/x86/cpu/idt.rs:41-57`):

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

The struct is `#[repr(C)]` and is **the contract between two languages**: if NASM's push order or the CPU's push order changes, this struct must change with it — otherwise the handler reads registers as other registers. (See the module docstring `src/arch/x86/cpu/idt.rs:1-9`.)

### The common handler trampoline (`arch/x86/asm/idt.asm:59-89`)

```asm
i686_ISR_common:
    pusha                      ; save all general registers
    mov ax, 0x10               ; reload data segments to kernel data
    mov ds/es/fs/gs, ax
    mov edi, esp               ; frame pointer survives the detour
    and esp, 0xFFFFFFF0        ; restore SysV 16-byte stack alignment —
    sub esp, 12                ; LLVM-emitted SSE (movaps) assumes it
    push edi                   ; argument: pointer to the frame (cdecl)
    call i686_ISR_handler      ; → Rust (src/arch/x86/cpu/idt.rs)
    add esp, 4                 ; drop the argument
    mov esp, edi               ; back to the interrupt frame
    popa                       ; restore registers
    add esp, 8                 ; drop int_num + error_code
    iret                       ; restore eip/cs/eflags (and error code) atomically
```

The realignment exists because exception entry (12 bytes) + stub pushes + `pusha` leave `esp` misaligned relative to the SysV i386 ABI; the aligned scratch sits below the frame image, which is free kernel-stack space. (`entry.asm` also initializes CR0/CR4 so SSE does not raise #UD in the first place.)

`iret` pops `eip`, `cs`, `eflags` — and the error code is consumed by it, which is why the cleanup `add esp, 8` only removes the two stub-pushed dwords.

### Registration (`src/arch/x86/cpu/idt.rs:184-212`)

`idt_init` — the symbol `_start` calls — fills gates 0..31 with `make_gate(handler as u32, 0x08, 0x8E)` via the `set_gate` helper, then executes `lidt` through inline assembly:

```rust
core::arch::asm!("lidt [{}]", in(reg) &desc, options(nostack, preserves_flags));
```

A single 3-byte instruction; no NASM round-trip needed. `make_gate` is a `const fn` that never touches the global table — encoding is pure and host-testable (`src/arch/x86/cpu/idt.rs:225-236`).

### Dispatch and the per-vector policy (`src/arch/x86/cpu/idt.rs`, `src/kernel/interrupts/exceptions.rs`)

`i686_ISR_handler` null-guards the ASM-passed `*mut InterruptFrame`, then dispatches on `int_num` (named constants pin vectors 0, 1 and 3; host tests lock them). Handlers migrate vector by vector — the source shows the exact current wiring — but the policy each one follows is fixed:

**Return only when resuming is meaningful: `#DB` (1) and `#BP` (3). Everything else: dump + halt.**

For a *fault*, `iret` re-executes the faulting instruction — with no paging, no user mode and no consumer of `into`/`bound`/FPU, resuming means an infinite exception loop. Traps (`#DB`/`#BP`) resume past the instruction, which is exactly what single-step and `int3` debugging need.

| Vectors | Class | Behavior |
|---|---|---|
| 1 `#DB`, 3 `#BP` | trap | dump + **return** |
| 0 `#DE`, 2 NMI, 4 `#OF`, 5 `#BR`, 6 `#UD`, 7 `#NM`, 9 `#CSO`, 16 `#MF`, 18 `#MC`, 19 `#XM`, 20 `#VE`, 21 `#CP`, 15/22–29/31 reserved | fatal, no error code | dump + halt |
| 8 `#DF`, 10 `#TS`, 11 `#NP`, 12 `#SS`, 13 `#GP`, 14 `#PF`, 17 `#AC`, 30 `#SX` | fatal, CPU pushes an error code | dump (+ `ERR=`) + halt |

`ERR=` is printed only for the error-code group: elsewhere the NASM stub pushes a dummy `0` that would be misleading. Reserved vectors stay reachable via `int $n` (every gate is PRESENT); NMI ignores `cli`. Known limitation: `#DF` runs on the same stack that may have caused the fault, so the dump itself can fault again (triple fault) — fixing it needs a task gate / TSS (plan.md Phase 5).

All dumps flow through one shared helper, `dump_exception` (`exceptions.rs`), fed by the pure, host-tested `build_dump_text`: it composes the single `putstr` payload (`Title\nEIP=\nCS=\nEFLAGS=` on rows 0–3, plus `ERR=` on row 4), then `put_hex_at(v, row, col)` (`src/kernel/drivers/vga.rs:147`) writes each `0xXXXXXXXX` value at the end of its label (`(1,4)`, `(2,3)`, `(3,7)`, `(4,4)`). Fields are copied to locals before the volatile MMIO writes; fatal handlers end in the shared `halt()` (`cli` + `hlt` loop). Hex text is built by the pure `format_hex` helper (`vga.rs:66-78`), which — like `support.rs` — uses only `while` byte loops so LLVM never emits a `memcpy` call on the exception path.

FFI note: the handler keeps its `extern "C"` ABI for the NASM `call`, so Clippy's `not_unsafe_ptr_arg_deref` is suppressed locally with a `SAFETY` justification instead of marking it `unsafe fn`.

## Why we did this

- **Stub-in-ASM, logic-in-Rust.** The stack surgery (`pusha`, dummy pushes, segment reloads) must be cycle-exact assembly; everything above that line — the C-equivalent handler — is Rust. The seam is one `call` with a pointer argument.
- **Uniform frame layout.** Pushing a dummy error code for vectors 0–7/9/15–29 makes every exception arrive with an identical frame shape. One `InterruptFrame` type, one handler signature — no per-vector `if` on the C side.
- **cfg(test) stubs.** NASM objects don't participate in the host test link, so a macro generates no-op `i686_ISR0..31` symbols for tests (`src/arch/x86/cpu/idt.rs:111-130`) — letting the gate-encoding logic be unit-tested without an emulator. (See [rust-for-osdev.md](rust-for-osdev.md#testing-without-hardware).)

## Why it matters

Exceptions are **how the CPU talks to the operating system**: a page fault, a divide-by-zero, a GP fault — all arrive through this exact pipeline. Every driver interrupt and every syscall entry will reuse the machinery built here. The two universal lessons:

1. **Frame discipline.** Interrupt code is stack bookkeeping; one wrong `push`/`pop` and `iret` returns to a corrupted context. The `InterruptFrame` struct *is* the specification of that bookkeeping.
2. **Bring-up ordering.** Vectors migrate from no-op to dump-then-halt (or dump-and-return) handlers group by group, before interrupts or user mode exist — it converts "triple fault, QEMU restarts" into "exception happened, here is the frame". Observable failure beats silent failure.

## Going deeper

- [Intel SDM Vol. 3, Chapter 6](https://www.intel.com/sdm) — exception and interrupt reference, error codes per vector
- [OSDev Wiki — IDT](https://wiki.osdev.org/IDT) — gate format and loading
- [OSDev Wiki — Interrupts](https://wiki.osdev.org/Interrupts) and [Exceptions](https://wiki.osdev.org/Exceptions) — vector map, error-code semantics
- [Writing an OS in Rust — Interrupts](https://os.phil-opp.com/cpu-exceptions/) — the modern long-mode version of this same machinery
