[bits 32]

extern i686_ISR_handler

section .text

; Stub without CPU error code: pushes dummy 0 + number, preserving the
; InterruptFrame { ..., int_num, error_code, eip, cs, eflags } layout expected by Rust.
%macro ISR_NOERRCODE 1
global i686_ISR%1
i686_ISR%1:
    push byte 0
    push byte %1
    jmp i686_ISR_common
%endmacro

; Stub with CPU error code (vectors 8,10-14,17): only pushes the number.
; Pushing a dummy here would misalign the frame by 4 bytes.
%macro ISR_ERRCODE 1
global i686_ISR%1
i686_ISR%1:
    push byte %1
    jmp i686_ISR_common
%endmacro

ISR_NOERRCODE 0    ; Divide Error
ISR_NOERRCODE 1    ; Debug
ISR_NOERRCODE 2    ; NMI
ISR_NOERRCODE 3    ; Breakpoint
ISR_NOERRCODE 4    ; Overflow
ISR_NOERRCODE 5    ; Bound Range
ISR_NOERRCODE 6    ; Invalid Opcode
ISR_NOERRCODE 7    ; Device Not Available
ISR_ERRCODE   8    ; Double Fault (CPU pusha error)
ISR_NOERRCODE 9    ; Coprocessor Segment Overrun
ISR_ERRCODE   10   ; Invalid TSS
ISR_ERRCODE   11   ; Segment Not Present
ISR_ERRCODE   12   ; Stack-Segment Fault
ISR_ERRCODE   13   ; General Protection
ISR_ERRCODE   14   ; Page Fault
ISR_NOERRCODE 15   ; Reserved
ISR_NOERRCODE 16   ; x87 FPU Error
ISR_ERRCODE   17   ; Alignment Check
ISR_NOERRCODE 18   ; Machine Check
ISR_NOERRCODE 19   ; SIMD FP
ISR_NOERRCODE 20   ; Virtualization
ISR_NOERRCODE 21   ; Control Protection (kept NOERR: see note)
ISR_NOERRCODE 22
ISR_NOERRCODE 23
ISR_NOERRCODE 24
ISR_NOERRCODE 25
ISR_NOERRCODE 26
ISR_NOERRCODE 27
ISR_NOERRCODE 28
ISR_NOERRCODE 29
ISR_ERRCODE   30   ; Security Exception (has error code on newer CPUs)
ISR_NOERRCODE 31

global i686_ISR_common
i686_ISR_common:
    pusha

    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; Give the Rust handler a stack that honors the SysV i386 16-byte
    ; alignment: the interrupt frame does not preserve it, and LLVM-emitted
    ; SSE (movaps on esp-relative locals) would fault with #GP otherwise.
    ; `edi` (callee-saved, and restored by popa anyway) holds the frame
    ; pointer across the detour; the aligned scratch sits below the frame
    ; image, which is free kernel-stack space.
    mov edi, esp
    and esp, 0xFFFFFFF0
    sub esp, 12
    push edi
    call i686_ISR_handler
    add esp, 4

    mov esp, edi
    popa
    add esp, 8
    iret
