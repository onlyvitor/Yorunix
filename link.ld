OUTPUT_FORMAT("elf32-i386")
ENTRY(_start)

SECTIONS {
    . = 1M;  /* Load kernel at 1MB (GRUB standard) */

    /* Dedicated section + KEEP: the Multiboot header must be within the first
       8 KiB of the binary for GRUB to recognize it. Without KEEP, --gc-sections may
       discard it or reorder it after the Rust .text (breaking boot). */
    .multiboot : { KEEP(*(.multiboot)) }
    .text : { *(.text*) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss  : { *(COMMON) *(.bss*) }
}