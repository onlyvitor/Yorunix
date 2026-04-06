#pragma once

#include <stdint.h>

#define IDT_PRESENT       0x80
#define IDT_RING0         0x00
#define IDT_RING3         0x60
#define IDT_TYPE_INTERRUPT_GATE 0x0E
#define IDT_TYPE_TRAP_GATE      0x0F

#define IDT_ENTRY(attr) (IDT_PRESENT | IDT_RING0 | (attr))

void idt_init(void);
