section .multiboot
align 4 ; multiboot header precisa ter um alinhamento de 4 bytes para ser reconhecido corretamente pelo bootloader
MBOOT_MAGIC  equ 0x1BADB002
MBOOT_FLAGS  equ 0x0
MBOOT_CHECKSUM equ -(MBOOT_MAGIC + MBOOT_FLAGS)

dd MBOOT_MAGIC
dd MBOOT_FLAGS
dd MBOOT_CHECKSUM

section .bss
align 16 ; alinha a pilha em um limite de 16 bytes para melhor desempenho possivel
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