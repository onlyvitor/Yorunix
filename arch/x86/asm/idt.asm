[bits 32]

extern i686_ISR_handler

section .text

; Stub sem error-code da CPU: empilha dummy 0 + número, preservando o layout
; InterruptFrame { ..., int_num, error_code, eip, cs, eflags } esperado pelo Rust.
%macro ISR_NOERRCODE 1
global i686_ISR%1
i686_ISR%1:
    push byte 0
    push byte %1
    jmp i686_ISR_common
%endmacro

; Stub com error-code da CPU (vetores 8,10-14,17): só empilha o número.
; Empilhar dummy aqui desalinharia o frame em 4 bytes.
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
ISR_NOERRCODE 21   ; Control Protection (mantido NOERR: ver nota)
ISR_NOERRCODE 22
ISR_NOERRCODE 23
ISR_NOERRCODE 24
ISR_NOERRCODE 25
ISR_NOERRCODE 26
ISR_NOERRCODE 27
ISR_NOERRCODE 28
ISR_NOERRCODE 29
ISR_ERRCODE   30   ; Security Exception (possui error-code em CPUs novas)
ISR_NOERRCODE 31

global i686_ISR_common
i686_ISR_common:
    pusha

    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    push esp
    call i686_ISR_handler
    add esp, 4

    popa
    add esp, 8
    iret
