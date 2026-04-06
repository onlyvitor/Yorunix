extern i686_ISR_handler

section .text

%macro ISR_NOERRCODE 1
global i686_ISR%1
i686_ISR%1:
    push byte 0
    push byte %1
    jmp i686_ISR_common
%endmacro

ISR_NOERRCODE 0
ISR_NOERRCODE 1

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
