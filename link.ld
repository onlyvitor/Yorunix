OUTPUT_FORMAT("elf32-i386")
ENTRY(_start)

SECTIONS {
    . = 1M;  /* Load kernel at 1MB (GRUB standard) */

    /* Seção dedicada + KEEP: o header Multiboot precisa estar nos primeiros
       8 KiB do binário para o GRUB reconhecer. Sem KEEP, --gc-sections pode
       descartá-lo ou reordená-lo para depois do .text do Rust (quebra boot). */
    .multiboot : { KEEP(*(.multiboot)) }
    .text : { *(.text*) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss  : { *(COMMON) *(.bss*) }
}