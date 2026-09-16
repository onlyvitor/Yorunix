# YoRunix Architecture Plan

This file records the agreed architectural direction for YoRunix. It does not implement the work. It exists so implementation can proceed in dependency order without changing unrelated files or expanding scope prematurely.

## 1. Current state

YoRunix is currently:

- A bootable 32-bit x86 kernel.
- A Rust `no_std`/`no_main` static library linked with NASM objects.
- Multiboot-compatible through GRUB or QEMU.
- Using a flat ring-0 GDT.
- Using an IDT with CPU exception vectors 0–31.
- Able to print through VGA text mode.
- Ending in a controlled `hlt` loop.

YoRunix is not yet a functioning microkernel. It has no:

- Serial console.
- Hardware IRQ support.
- PIC remapping.
- Timer driver.
- Multiboot memory-map parsing.
- Physical frame allocator.
- Paging.
- Kernel heap.
- User mode.
- TSS.
- Syscall interface.
- Scheduler.
- Process abstraction.
- IPC mechanism.
- Block-device driver.
- Filesystem.
- Network stack.

The intended architecture is therefore best described at this stage as a **microkernel foundation**, not an operational microkernel.

## 2. Baseline assumptions

- Keep the current 32-bit i686 architecture for the next milestones.
- Keep the existing Rust/NASM static-library linking model.
- Keep the existing boot path and descriptor-table contracts unless a phase explicitly changes them.
- Do not claim stability, security, or user-service isolation until those boundaries are implemented and tested.
- Keep implementation slices small and reviewable.
- Do not mix unrelated metadata changes with kernel milestones.

## 3. Dependency order

Implementation should follow this dependency order:

```text
serial and panic diagnostics
        ↓
exception dispatch
        ↓
PIC, IRQs, and timer
        ↓
Multiboot memory map and frame allocator
        ↓
paging and kernel heap
        ↓
user mode, tasks, and syscalls
        ↓
IPC
        ↓
kernel drivers and filesystem
```

Serial and exception diagnostics come first because later paging, process, and syscall failures would otherwise be invisible. Hardware interrupts come next because timer-driven scheduling and device input depend on them. Memory management precedes processes because allocation and isolation depend on knowing usable RAM and controlling page tables. IPC comes after processes because it needs tasks, address spaces, and system calls. Drivers and filesystem remain last because they depend on almost every earlier layer.

## 4. Phase 0 — Separate housekeeping

### 4.1 Objective

Establish a clean repository baseline without changing kernel behavior.

### 4.2 Why it matters

The working tree has uncommitted changes in `Cargo.toml`, `Cargo.lock`, and `src/lib.rs`. Those changes concern package version, license text, and a greeting. They are unrelated to the architectural roadmap and should not be mixed into kernel milestones.

### 4.3 Work

- Decide whether the intended package version is `0.0.1` or `0.1.0`.
- Align the license metadata with `LICENSE`.
- If BSD 3-Clause is intended, prefer the normal SPDX expression:

```text
BSD-3-Clause
```

- Use one consistent project spelling.
- Commit or stash these housekeeping changes separately before starting Phase 1.

### 4.4 Files

- `Cargo.toml`
- `Cargo.lock`
- `src/lib.rs`
- `LICENSE`
- `README.md`, only if documentation needs a matching correction.

### 4.5 Acceptance criteria

- No uncommitted unrelated changes remain before architectural work begins.
- Package metadata is internally consistent.
- `cargo test` still passes.
- `cargo clippy --all-targets -- -D warnings` still passes.
- `cargo clippy --target i686-unknown-linux-gnu -- -D warnings` still passes.
- `make` still builds successfully.

## 5. Phase 1 — Observable kernel and exception diagnostics

### 5.1 Objective

Make boot failures, panics, and CPU exceptions visible and testable.

### 5.2 Why this is first

The current exception handler intentionally does nothing. A fault can return through `iret` to the faulting instruction and loop without explaining what happened. The panic handler halts without printing anything. The only output path is VGA, while automated testing would benefit from serial output.

Every later milestone depends on accurate low-level diagnostics.

### 5.3 Work

- Add a COM1 serial-port driver.
- Encapsulate port input/output and volatile access inside that driver.
- Initialize serial output during early kernel initialization.
- Print a deterministic boot marker, for example:

```text
YoRunix boot OK
```

- Route panic output through serial before entering the halt loop.
- Extend `i686_ISR_handler` to dispatch by `frame.int_num`.
- Give each supported exception a stable name.
- Print fatal-exception information through serial.
- Halt after reporting a fatal exception instead of silently returning.
- Preserve the existing NASM-generated interrupt-frame layout.

### 5.4 Files

New implementation file:

- `src/serial.rs`

Existing implementation files:

- `src/lib.rs`
- `src/idt.rs`

Existing low-level contract:

- `arch/x86/idt.asm`

Build and automation:

- `Makefile`
- `.github/workflows/ci.yml`

Optional documentation:

- `docs/`
- `README.md`

### 5.5 Behavior requirements

- Normal boot prints the boot marker once.
- Panic prints a recognizable panic marker before halting.
- A deliberately triggered CPU exception prints its vector and relevant frame information.
- Fatal exceptions do not silently resume the faulting instruction.

### 5.6 Tests

- Host unit tests for exception-vector names and dispatch decisions.
- Host tests for any pure serial formatting or buffering logic.
- QEMU integration behavior:
  - Boot emits the expected serial marker.
  - A synthetic exception emits the expected diagnostic output.
- CI should assert serial output instead of only asserting that QEMU stays alive.

### 5.7 Out of scope

- No PIC remapping.
- No hardware IRQ drivers.
- No keyboard driver.
- No memory allocator.
- No paging.
- No processes.

## 6. Phase 2 — Hardware interrupts and timer

### 6.1 Objective

Establish the first real hardware interrupt path using the PIC and PIT.

### 6.2 Why this comes second

The IDT already contains CPU exception gates. Hardware interrupts are the next natural extension. A timer provides the future scheduler tick and gives integration tests a recurring observable hardware event.

Interrupts must not be enabled before the PIC and IRQ gates are ready. IRQ0 currently conflicts with CPU exception vector 8, so PIC remapping is mandatory before `sti`.

### 6.3 Work

- Add an 8259 PIC driver.
- Remap the master PIC to vectors `0x20–0x27`.
- Remap the slave PIC to vectors `0x28–0x2F`.
- Mask all IRQs during initialization.
- Unmask only the timer after its driver is ready.
- Add NASM IRQ stubs for vectors 32–47.
- Reuse the existing interrupt-frame layout.
- Register IRQ gates in the Rust IDT.
- Add a PIT timer driver for IRQ0.
- Maintain and expose a tick counter.
- Send an end-of-interrupt before returning from each IRQ handler.
- Enable interrupts only after initialization is complete.
- Keep the IRQ0 handling minimal at first.

### 6.4 Files

New implementation files:

- `src/pic.rs`
- `src/timer.rs`

Existing implementation files:

- `src/idt.rs`
- `src/lib.rs`
- `boot/entry.asm`, if initialization sequencing changes

Low-level files:

- `arch/x86/idt.asm`

Build and automation:

- `Makefile`
- `.github/workflows/ci.yml`

### 6.5 Behavior requirements

- Timer interrupts increment the tick counter.
- IRQ handling sends the correct end-of-interrupt.
- No spurious reboot or triple fault occurs when interrupts are enabled.
- CPU exception handling remains intact.
- Serial output remains usable from IRQ context where safe.

### 6.6 Tests

- Host unit tests for PIC command construction where pure logic exists.
- Host unit tests for timer divisor and tick accounting where pure logic exists.
- QEMU integration behavior:
  - Timer ticks increase over time.
  - Serial output shows periodic tick information.
  - No unexpected reboot occurs.

### 6.7 Out of scope

- Keyboard driver.
- Preemptive scheduling.
- Process management.
- Memory allocation.
- Filesystem work.

## 7. Phase 3 — Physical memory discovery and allocation

### 7.1 Objective

Discover usable physical RAM and allocate fixed-size physical frames safely.

### 7.2 Why this comes third

Paging, kernel heap, user address spaces, and process stacks all depend on knowing which physical memory is usable and which memory is reserved. The current boot configuration does not request a memory map, so this information is unavailable.

This phase can be researched in parallel with Phase 2, but integration should wait until exception and interrupt diagnostics are stable.

### 7.3 Work

- Request the Multiboot memory map by changing `MBOOT_FLAGS`.
- Specifically request the memory-map field using Multiboot flag bit 6 (`0x40`).
- Preserve the Multiboot information address currently provided in `EBX`.
- Pass that address explicitly to `kernel_main`.
- Parse Multiboot memory-map entries.
- Validate entry sizes, addresses, and types.
- Distinguish usable RAM from reserved or hardware-owned memory.
- Reserve:
  - Kernel image.
  - Boot stack.
  - GDT and IDT.
  - VGA buffer.
  - Multiboot information and memory-map structures.
  - BIOS and other reserved regions.
- Add linker symbols for kernel boundaries in `link.ld`.
- Implement a physical frame allocator.
- Prefer a bitmap or another explicitly testable allocation structure.
- Track free, allocated, and reserved frames.
- Print the memory map and allocator state through serial output.

### 7.4 Files

Existing boot and linking files:

- `boot/entry.asm`
- `link.ld`

New implementation files:

- `src/multiboot.rs`
- `src/memory.rs`
- Optionally `src/memory/frame_allocator.rs`

Existing implementation files:

- `src/lib.rs`

### 7.5 Behavior requirements

- The kernel receives and validates the Multiboot information address.
- Usable and reserved memory regions are reported.
- The kernel image is never allocated as free RAM.
- Allocation of the same frame twice is impossible.
- Invalid memory-map input cannot cause silent memory corruption.

### 7.6 Tests

- Host unit tests for Multiboot entry parsing.
- Host unit tests for frame allocation, freeing, double allocation, and reservation.
- Host unit tests for boundary conditions.
- QEMU integration behavior:
  - Serial output shows the parsed memory map.
  - Serial output shows allocator initialization.
  - No page allocator is required yet.

### 7.7 Out of scope

- Page tables.
- Enabling paging.
- Kernel heap.
- Per-process address spaces.
- User mode.

## 8. Phase 4 — Paging and kernel heap

### 8.1 Objective

Enable controlled virtual memory and provide safe kernel dynamic allocation.

### 8.2 Why this order matters

A physical frame allocator tells the kernel what RAM can be used. Paging tells the CPU how each page can be accessed. A kernel heap depends on both, especially as process metadata and scheduler structures grow.

Page faults must also be diagnosable before paging is enabled, which is another reason Phase 1 comes first.

### 8.3 Work

- Define page-directory and page-table entry structures.
- Add compile-time and unit-test coverage for entry encoding.
- Identity-map the kernel image, VGA buffer, and other required low-memory regions.
- Load the initial page directory into `CR3`.
- Enable paging through `CR0`.
- Invalidate stale TLB entries where required.
- Add a page-fault handler path through the existing exception dispatch infrastructure.
- Print useful page-fault diagnostics.
- Validate paging with a QEMU boot before introducing a heap.
- Add a kernel heap after physical allocation and paging are stable.
- Expose the heap through `#[global_allocator]`.
- Keep early allocation simple before introducing a production-style allocator.

### 8.4 Files

New implementation files:

- `src/paging.rs`
- `src/alloc.rs`
- Optionally `src/memory/paging.rs`

Existing implementation files:

- `src/lib.rs`
- `src/memory.rs`
- `src/idt.rs`
- `link.ld`

### 8.5 Behavior requirements

- Paging can be enabled without losing serial, VGA, stack, GDT, or IDT access.
- Page-fault diagnostics identify the faulting vector and cause.
- Invalid page accesses do not silently corrupt adjacent memory.
- Heap allocations return valid memory within allocator-owned regions.
- Out-of-memory behavior is explicit rather than silent.

### 8.6 Tests

- Host unit tests for page-directory and page-table encoding.
- Host unit tests for frame-to-page mapping logic.
- Host unit tests for heap allocation and boundary behavior.
- QEMU integration behavior:
  - Kernel boots with paging enabled.
  - A deliberately invalid access produces a page-fault diagnostic.
  - A basic heap smoke test succeeds.

### 8.7 Out of scope

- Per-process address spaces.
- Ring-3 processes.
- System calls.
- IPC.
- Filesystem work.

## 9. Phase 5 — Tasks, user mode, and system calls

### 9.1 Objective

Create the first real privilege boundary and the first executable user task.

### 9.2 Why this is the microkernel turning point

Until this phase, every instruction runs in ring 0. A project cannot meaningfully be called a microkernel until at least one task runs outside the kernel privilege level and communicates through a controlled interface.

This phase should not begin until memory discovery, paging, heap behavior, timer interrupts, and exception diagnostics are stable.

### 9.3 Work

- Define the process and task model explicitly before writing scheduler code.
- Add ring-3 code and data descriptors to the GDT.
- Add a task-state segment for privilege transitions.
- Maintain a valid kernel stack for interrupts arriving from user mode.
- Implement task and process control structures.
- Start with a fixed-size task table to avoid depending on dynamic allocation prematurely.
- Add one ring-3 user task before adding a general scheduler.
- Add a simple user binary or user entry point.
- Implement context switching in architecture-specific assembly.
- Preserve and restore CPU state correctly across switches.
- Switch address spaces through `CR3` where applicable.
- Add a controlled syscall interface.
- Validate syscall arguments.
- Never dereference untrusted user pointers directly in the kernel.
- Add cooperative scheduling first.
- Add timer-driven preemption after the basic task boundary works.

### 9.4 Files

New implementation files:

- `src/task.rs`
- `src/process.rs`
- `src/scheduler.rs`
- `src/syscall.rs`
- `src/tss.rs`

New low-level files:

- `arch/x86/context.asm`
- `arch/x86/syscall.asm`, if a separate assembly entry point is needed

Existing implementation files:

- `src/gdt.rs`
- `src/idt.rs`
- `src/paging.rs`
- `src/memory.rs`
- `src/alloc.rs`
- `src/lib.rs`

New user-space area:

- `user/`
- User build configuration and linker script

Build and automation:

- `Makefile`
- `.github/workflows/ci.yml`

### 9.5 Behavior requirements

- Kernel code and data remain inaccessible from user mode.
- Interrupt handling switches to a valid kernel stack on privilege transitions.
- A ring-3 task can execute without corrupting kernel state.
- A defined syscall can return a result to user mode.
- Invalid user-mode accesses produce controlled faults.
- Task state transitions are explicit and testable.

### 9.6 Tests

- Host unit tests for task-state transitions.
- Host unit tests for syscall argument validation.
- Host unit tests for scheduler data structures.
- QEMU integration behavior:
  - A ring-3 task starts.
  - A syscall returns successfully.
  - A faulting user task does not crash the kernel.
  - Timer-driven switching works when preemption is enabled.

### 9.7 Out of scope

- Full multiuser process semantics.
- IPC beyond the minimum needed for testing syscalls.
- User-space drivers.
- Filesystem services.
- Network services.

## 10. Phase 6 — Interprocess communication

### 10.1 Objective

Allow isolated tasks to communicate through the kernel without sharing memory unsafely.

### 10.2 Why this comes after tasks

IPC needs:

- Runnable tasks.
- Separate stack state.
- Address-space awareness.
- Scheduler wait queues.
- A secure syscall interface.

Implementing IPC before those pieces exist would force unsafe shortcuts and undermine the intended microkernel boundary.

### 10.3 Work

- Define the IPC model before implementation.
- Start with fixed-size kernel-mediated messages.
- Avoid arbitrary shared memory in the first implementation.
- Define ports, endpoints, mailboxes, or another explicit addressing model.
- Copy message data through the kernel rather than trusting user pointers.
- Integrate blocking and wakeup behavior with the scheduler.
- Add message-related syscalls.
- Validate process identifiers, message sizes, and buffers.
- Add one simple user-space test service.
- Keep the first service minimal, such as an echo, counter, or logging service.

### 10.4 Files

New implementation files:

- `src/ipc.rs`
- `src/message.rs`
- Possibly `src/service.rs`

Existing implementation files:

- `src/syscall.rs`
- `src/scheduler.rs`
- `src/process.rs`
- `src/task.rs`
- `src/paging.rs`
- `src/alloc.rs`

User-space files:

- `user/`
- User-space message or syscall wrappers

### 10.5 Behavior requirements

- Two isolated user tasks can exchange messages.
- Invalid process identifiers are rejected.
- Invalid buffers are rejected.
- Message data is copied safely between protection domains.
- Blocking and wakeup behavior works correctly.
- A faulty user task cannot corrupt kernel memory through IPC.

### 10.6 Tests

- Host unit tests for message encoding and validation.
- Host unit tests for wait-queue transitions.
- QEMU integration behavior:
  - Successful message exchange.
  - Rejected invalid process identifier.
  - Rejected invalid buffer.
  - No kernel crash from a faulty user task.

### 10.7 Out of scope

- Arbitrary shared memory.
- Zero-copy IPC optimization.
- Capability-based security framework.
- Filesystem services.
- Device-driver services, except where needed for the minimal test service.

## 11. Phase 7 — Device drivers and filesystem

### 11.1 Objective

Move from a kernel with tasks and IPC toward useful user-visible services.

### 11.2 Why this remains last

Filesystem and network work requires:

- Stable interrupts.
- Stable memory management.
- Task isolation.
- Controlled system calls.
- A working IPC path.

Starting a filesystem earlier would produce a kernel-space filesystem without the intended microkernel structure, process isolation, or service boundary.

### 11.3 Work

- Add a keyboard driver after serial, PIC, and timer support.
- Add a block-device driver before filesystem work.
- Define a minimal virtual filesystem interface.
- Add a simple filesystem implementation.
- Decide whether the first filesystem is persistent or memory-backed.
- Avoid calling an in-memory test filesystem a complete filesystem implementation.
- Add device discovery only as needed.
- Keep drivers small and reviewable.
- Move appropriate services to user space only after IPC is stable.

### 11.4 Files

Likely new areas:

- `src/drivers/keyboard.rs`
- `src/drivers/block.rs`
- `src/fs/vfs.rs`
- `src/fs/simplefs.rs`
- User-space service implementations under `user/`

Existing files may require changes only where hardware, syscall, scheduler, or IPC contracts demand them.

### 11.5 Behavior requirements

- Keyboard input reaches user space through the intended driver path.
- Block reads and writes behave deterministically.
- Filesystem metadata remains consistent across basic operations.
- User-space access to devices and files passes through controlled interfaces.
- Faulty services cannot bypass kernel protection boundaries.

### 11.6 Tests

- Host unit tests for filesystem structures and pure parsing logic.
- Host unit tests for scheduler and IPC integration points.
- QEMU integration behavior for keyboard, block I/O, and filesystem operations.
- CI coverage for every newly stabilized behavior.

### 11.7 Out of scope for the current plan

- Network stack.
- USB subsystem.
- Advanced storage drivers.
- Multi-CPU scheduling.
- Production-grade filesystem features.

## 12. Testing strategy

### 12.1 Host unit tests

Use host tests for deterministic, hardware-independent logic:

- Descriptor and gate encoding.
- Multiboot parsing.
- Frame allocation.
- Page-table encoding.
- Heap behavior.
- Scheduler state transitions.
- Syscall validation.
- IPC validation.

### 12.2 QEMU integration tests

Use QEMU for behavior that cannot be proven on the host:

- Boot marker appears.
- Exception diagnostics appear.
- Timer ticks increase.
- PIC behavior does not cause reboot loops.
- Paging enables successfully.
- Page faults are reported.
- Ring-3 tasks start.
- Syscalls return expected values.
- IPC succeeds and rejects invalid input.

### 12.3 Regression rules

Every stabilized behavior should have a corresponding test. A fixes-only change should preferably add or update a test covering the fixed behavior. Integration tests should assert expected output wherever serial output is available.

## 13. CI and build integration

### 13.1 Continuous integration

The pipeline should evolve alongside the kernel:

- Continue enforcing formatting and Clippy checks.
- Continue running host unit tests.
- Add serial-output assertions as soon as the serial driver exists.
- Add timer assertions after the timer driver exists.
- Add memory, paging, syscall, and IPC assertions as each phase stabilizes.
- Continue uploading `kernel.bin` as an artifact.
- Consider uploading QEMU logs only on failure.

### 13.2 Build integration

The Makefile should gain explicit integration targets rather than overloading existing build targets. Likely additions include:

- Serial boot target.
- Integration-test target.
- Separate normal build, ISO build, and test flows.

New Rust modules must also be represented correctly in Makefile dependencies or through a dependable wildcard/source-discovery mechanism.

## 14. Risks and deliberate non-goals

### 14.1 Risks

- Debugging without serial output is unreliable.
- Enabling interrupts before PIC remapping can route hardware IRQs into CPU exception vectors.
- Enabling paging before page-fault diagnostics are stable can make failures unobservable.
- User mode without TSS and valid kernel stacks can corrupt kernel state.
- Dereferencing user pointers directly in the kernel can break isolation.
- Exact QEMU reset-count heuristics may vary across QEMU versions.
- `static mut` tables and raw-pointer access become riskier once interrupts and scheduling introduce concurrency.

### 14.2 Non-goals for the next implementation phases

- No 64-bit migration yet.
- No strict microkernel claim before user mode and IPC.
- No filesystem implementation before processes, syscalls, and IPC.
- No network stack.
- No APIC/SMP subsystem.
- No production allocator or filesystem optimization.
- No large cross-cutting refactoring without a concrete defect.
- No unrelated metadata changes mixed into kernel feature commits.

## 15. Acceptance checklist

The roadmap should be considered on track only when:

- [ ] Repository metadata is consistent and committed separately.
- [ ] Boot output is visible through serial.
- [ ] Panics and CPU exceptions report useful diagnostics.
- [ ] PIC, IRQs, and timer work without reboot loops.
- [ ] Usable memory is discovered and reserved correctly.
- [ ] Physical allocation is tested.
- [ ] Paging is enabled and page faults are reported.
- [ ] Kernel heap allocation is tested.
- [ ] At least one ring-3 task runs safely.
- [ ] System calls validate untrusted input.
- [ ] Isolated tasks can exchange IPC messages.
- [ ] Drivers and filesystem work are deferred until the above boundaries exist.
