 global gdt_init
 global gdt_flush

 gdt_init:
    null_descriptor:
        dq 0 ; Null descriptor (must be the first entry in the GDT)
    code_descriptor:
        dw 0xffff              ; Limit low
        dw 0x0000              ; Base low
        db 0x00
        db 0b10011010
        db 0b11001111
        db 0
    data_descriptor:
        dw 0xffff              ; Limit low
        dw 0x0000              ; Base low
        db 0x00
        db 0b10010010
        db 0b11001111
        db 0
gdt_end:
    gdt_descriptor:
        dw gdt_end - gdt_init - 1 ; Limit (size of GDT - 1)
        dd gdt_init                ; Base address of GDT

gdt_flush:
    lgdt [gdt_descriptor] ; Load the GDT
    mov eax, cr0         ; Data segment selector (index 2 in GDT)
    or eax, 1
    mov cr0, eax

    jmp 0x08:flush_done ; Code segment selector (index 1 in GDT)
flush_done:
    mov ax, 0x10    ; data segment selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    ret
