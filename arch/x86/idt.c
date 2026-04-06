#include "idt.h"

typedef struct {
    uint16_t base_low;
    uint16_t selector;
    uint8_t  zero;
    uint8_t  type_attr;
    uint8_t  base_high;
} __attribute__((packed)) IDTEntry;

typedef struct {
    uint16_t limit;
    uint32_t base;
} __attribute__((packed)) IDTDescriptor;

typedef struct {
    uint32_t edi, esi, ebp, esp_save, ebx, edx, ecx, eax;
    uint32_t int_num, error_code;
    uint32_t eip, cs, eflags;
} __attribute__((packed)) InterruptFrame;

static IDTEntry idt[256];
static IDTDescriptor idt_descriptor;

static void idt_set_gate(uint8_t num, uint32_t base, uint16_t selector, uint8_t attr)
{
    idt[num].base_low  = base & 0xFFFF;
    idt[num].base_high = (base >> 16) & 0xFF;
    idt[num].selector  = selector;
    idt[num].zero      = 0;
    idt[num].type_attr = attr;
}

void i686_ISR_handler(InterruptFrame* frame)
{
    (void)frame;
}

extern void i686_ISR0(void);
extern void i686_ISR1(void);

void idt_init(void)
{
    idt_descriptor.limit = sizeof(idt) - 1;
    idt_descriptor.base  = (uint32_t)&idt;

    idt_set_gate(0, (uint32_t)i686_ISR0, 0x08, IDT_ENTRY(IDT_TYPE_INTERRUPT_GATE));
    idt_set_gate(1, (uint32_t)i686_ISR1, 0x08, IDT_ENTRY(IDT_TYPE_INTERRUPT_GATE));

    __asm__ volatile ("lidt %0" : : "m"(idt_descriptor));
}
