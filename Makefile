BUILD_DIR =  build

CC = gcc
LD = ld
AS = nasm

ARCH := i386

ENTRY_POINT = boot/boot.asm
ENTRY_OBJ = $(BUILD_DIR)/boot.o

KERNEL_SRC = $(wildcard core/*.c)
KERNEL_OBJ = $(patsubst core/%.c, $(BUILD_DIR)/%.o, $(KERNEL_SRC))

LINKER_SCRIPT = link.ld

CFLAGS = -ffreestanding -nostdlib -fno-builtin -fno-stack-protector -Wall -Wextra -Werror -Iinclude

LINKER_FLAGS =
ASFLAGS =

ifeq ($(ARCH), x86_64)
 CFLAGS += -m64
 LINKER_FLAGS += -m elf_x86_64
 ASFLAGS += -f elf64
else
 CFLAGS += -m32
 LINKER_FLAGS += -m elf_i386
 ASFLAGS += -f elf32
endif

.PHONY: all clean run

all: $(BUILD_DIR)/kernel.bin

$(BUILD_DIR)/kernel.bin: $(ENTRY_OBJ) $(KERNEL_OBJ)
	$(LD) $(LINKER_FLAGS) -T $(LINKER_SCRIPT) -o $@ $^

$(ENTRY_OBJ): $(ENTRY_POINT)
	$(AS) $(ASFLAGS) -o $@ $<

$(BUILD_DIR)/%.o: core/%.c
	$(CC) $(CFLAGS) -c -o $@ $< -MMD -MF $(@:.o=.d)

clean:
	rm -rf $(BUILD_DIR)

run: $(BUILD_DIR)/kernel.bin
	qemu-system-x86_64 -kernel $<

$(shell mkdir -p $(BUILD_DIR))

-include $(wildcard $(BUILD_DIR)/*.d)