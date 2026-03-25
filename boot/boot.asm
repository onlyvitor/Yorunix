section .multiboot
align 4 ; multiboot header precisa ter um alinhamento de 4 bytes para ser reconhecido corretamente pelo bootloader
dd 0x1BADB002          ; Multiboot magic number
dd 0x0                 ; multiboot flags
dd -(0x1BADB002 + 0x0) ; Checksum

section .bss
align 16 ; alinha a pilha em um limite de 16 bytes para melhor desempenho possivel
stack_bottom: 
resb 16384           ; 16 KB stack
stack_top:

section .text
global _start
extern kernel_main 

_start:
 
 mov esp, stack_top  ; Set up stack
 call kernel_main    ; Call kernel main function
 hlt                 ; Halt CPU (loop forever)