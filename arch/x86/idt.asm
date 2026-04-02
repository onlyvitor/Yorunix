global idt_init
global idt_flush
global isr0
global isr1
global isr2
global isr3
global isr4
global isr5
global isr6
global isr7
global isr8
global isr9
global isr10
global isr11
global isr12
global isr13
global isr14
global isr15

extern isr_handler

; IDT entry structure: 8 bytes per entry
; Bytes 0-1: Offset 0-15 (base address of interrupt handler)
; Bytes 2-3: Segment selector (code segment from GDT)
; Byte 4: Flags (reserved, should be 0)
; Byte 5: Gate type and attributes
; Bytes 6-7: Offset 16-31 (rest of handler address)

idt_init:
    ; ISR 0 - Divide by Zero
    dw isr0 & 0xFFFF
    dw 0x08                    ; Segment selector (code segment from GDT)
    db 0x00                    ; Reserved
    db 0x8E                    ; Gate type: 32-bit interrupt gate
    dw (isr0 >> 16) & 0xFFFF

    ; ISR 1 - Debug Exception
    dw isr1 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr1 >> 16) & 0xFFFF

    ; ISR 2 - Non-Maskable Interrupt
    dw isr2 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr2 >> 16) & 0xFFFF

    ; ISR 3 - Breakpoint
    dw isr3 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr3 >> 16) & 0xFFFF

    ; ISR 4 - Overflow
    dw isr4 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr4 >> 16) & 0xFFFF

    ; ISR 5 - Bound Range Exceeded
    dw isr5 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr5 >> 16) & 0xFFFF

    ; ISR 6 - Invalid Opcode
    dw isr6 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr6 >> 16) & 0xFFFF

    ; ISR 7 - Device Not Available
    dw isr7 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr7 >> 16) & 0xFFFF

    ; ISR 8 - Double Fault
    dw isr8 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr8 >> 16) & 0xFFFF

    ; ISR 9 - Coprocessor Segment Overrun
    dw isr9 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr9 >> 16) & 0xFFFF

    ; ISR 10 - Invalid TSS
    dw isr10 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr10 >> 16) & 0xFFFF

    ; ISR 11 - Segment Not Present
    dw isr11 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr11 >> 16) & 0xFFFF

    ; ISR 12 - Stack Segment Fault
    dw isr12 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr12 >> 16) & 0xFFFF

    ; ISR 13 - General Protection Fault
    dw isr13 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr13 >> 16) & 0xFFFF

    ; ISR 14 - Page Fault
    dw isr14 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr14 >> 16) & 0xFFFF

    ; ISR 15 - Reserved
    dw isr15 & 0xFFFF
    dw 0x08
    db 0x00
    db 0x8E
    dw (isr15 >> 16) & 0xFFFF

idt_end:
    idt_descriptor:
        dw idt_end - idt_init - 1  ; Limit (size of IDT - 1)
        dd idt_init                ; Base address of IDT

idt_flush:
    lidt [idt_descriptor]   ; Load the IDT
    ret

; ISR stubs that push interrupt number and jump to common handler

isr0:
    push 0
    jmp isr_common

isr1:
    push 1
    jmp isr_common

isr2:
    push 2
    jmp isr_common

isr3:
    push 3
    jmp isr_common

isr4:
    push 4
    jmp isr_common

isr5:
    push 5
    jmp isr_common

isr6:
    push 6
    jmp isr_common

isr7:
    push 7
    jmp isr_common

isr8:
    push 8
    jmp isr_common

isr9:
    push 9
    jmp isr_common

isr10:
    push 10
    jmp isr_common

isr11:
    push 11
    jmp isr_common

isr12:
    push 12
    jmp isr_common

isr13:
    push 13
    jmp isr_common

isr14:
    push 14
    jmp isr_common

isr15:
    push 15
    jmp isr_common

isr_common:
    pusha                   ; Push all general purpose registers
    push ds                 ; Push data segment
    push es
    push fs
    push gs
    mov ax, 0x10            ; Load kernel data segment
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    call isr_handler        ; Call C handler
    pop gs                  ; Restore segment registers
    pop fs
    pop es
    pop ds
    popa                    ; Restore general purpose registers
    add esp, 4              ; Remove interrupt number from stack
    iret                    ; Return from interrupt
