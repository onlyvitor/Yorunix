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

    ; Initialize the x87/SSE state the compiler assumes: SeaBIOS/GRUB boot
    ; with CR0.EM=1 and CR4.OSFXSR=0, so any SSE instruction LLVM emits (e.g.
    ; a movaps zero-init of a local array) would raise #UD inside the kernel.
    mov eax, cr0
    and eax, 0xFFFFFFFB       ; CR0.EM (bit 2) = 0 — no math-emulator traps
    or eax, 0x00000002        ; CR0.MP (bit 1) = 1 — monitor coprocessor
    mov cr0, eax
    mov eax, cr4
    or eax, 0x00000600        ; CR4.OSFXSR (9) | OSXMMEXCPT (10): SSE usable,
    mov cr4, eax              ; SIMD FP exceptions route to #XM (vector 19)
    fninit                    ; reset x87 (masks all x87 exceptions)

    call i686_GDT_Initialize  ; Initialize GDT
    call idt_init             ; Initialize IDT
    call kernel_main          ; Call kernel main function
.hang:
    hlt
    jmp .hang