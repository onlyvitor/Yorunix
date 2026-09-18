section .multiboot
align 4 ; multiboot header must be 4-byte aligned to be correctly recognized by the bootloader
MBOOT_MAGIC  equ 0x1BADB002
MBOOT_FLAGS  equ 0x0
MBOOT_CHECKSUM equ -(MBOOT_MAGIC + MBOOT_FLAGS)

dd MBOOT_MAGIC
dd MBOOT_FLAGS
dd MBOOT_CHECKSUM

section .bss
align 16 ; align the stack to a 16-byte boundary for best possible performance
stack_bottom:
resb 16384           ; 16 KB stack
global stack_top
stack_top:

section .text
global _start
extern kernel_main
extern i686_GDT_Initialize
extern idt_init

_start:

    cli
    mov esp, stack_top        ; Set up stack pointer
    call i686_GDT_Initialize  ; Initialize GDT
    call idt_init             ; Initialize IDT
    call kernel_main          ; Call kernel main function
.hang:
    hlt
    jmp .hang