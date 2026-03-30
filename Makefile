BUILD_DIR =  build

CC = gcc
LD = ld
AS = nasm

ARCH := i386

ENTRY_POINT = boot/entry.asm
ENTRY_OBJ = $(BUILD_DIR)/entry.o

GDT_SRC = arch/x86/gdt.asm
GDT_OBJ = $(BUILD_DIR)/gdt.o

# Drivers sources/objects
DRV_SRC = $(wildcard drivers/*.c)
DRV_OBJ = $(patsubst drivers/%.c,$(BUILD_DIR)/%.o,$(DRV_SRC))

KERNEL_SRC = $(wildcard core/*.c)
KERNEL_OBJ = $(patsubst core/%.c, $(BUILD_DIR)/%.o, $(KERNEL_SRC))

LINKER_SCRIPT = link.ld

# ISO / GRUB
ISO_DIR = iso
GRUB_CFG = boot/grub/grub.cfg

CFLAGS = -ffreestanding -nostdlib -fno-builtin -fno-stack-protector -Wall -Wextra -Werror -Iinclude

LINKER_FLAGS = -z noexecstack
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

$(BUILD_DIR)/kernel.bin: $(ENTRY_OBJ) $(GDT_OBJ) $(KERNEL_OBJ) $(DRV_OBJ)
	$(LD) $(LINKER_FLAGS) -T $(LINKER_SCRIPT) -o $@ $^

$(ENTRY_OBJ): $(ENTRY_POINT)
	$(AS) $(ASFLAGS) -o $@ $<

$(GDT_OBJ): $(GDT_SRC)
	$(AS) $(ASFLAGS) -o $@ $<

$(BUILD_DIR)/%.o: core/%.c
	$(CC) $(CFLAGS) -c -o $@ $< -MMD -MF $(@:.o=.d)

$(BUILD_DIR)/%.o: drivers/%.c
	$(CC) $(CFLAGS) -c -o $@ $< -MMD -MF $(@:.o=.d)

clean:
	rm -rf $(BUILD_DIR)
	rm -rf $(ISO_DIR)

run: $(BUILD_DIR)/kernel.bin
	qemu-system-x86_64 -kernel $<

$(shell mkdir -p $(BUILD_DIR))

-include $(wildcard $(BUILD_DIR)/*.d)

.PHONY: iso prepare-iso run-grub grub-install-instructions

iso: $(BUILD_DIR)/kernel.bin
	@rm -rf $(ISO_DIR)
	@mkdir -p $(ISO_DIR)/boot/grub
	@cp $^ $(ISO_DIR)/boot/kernel.bin
	@cp $(GRUB_CFG) $(ISO_DIR)/boot/grub/grub.cfg
	@echo "Creating ISO with grub-mkrescue..."
	@grub-mkrescue -o $(BUILD_DIR)/kernel.iso $(ISO_DIR) || (echo "grub-mkrescue not found. Run 'make grub-install-instructions' to see install commands." && false)

run-grub: iso
	qemu-system-x86_64 -cdrom $(BUILD_DIR)/kernel.iso -boot d

grub-install-instructions:
	@echo "Install grub-mkrescue and xorriso (examples):"
	@echo "  Debian/Ubuntu: sudo apt update && sudo apt install grub2-common xorriso"
	@echo "  Fedora: sudo dnf install grub2-efi-x64 xorriso"
	@echo "  Arch: sudo pacman -S grub xorriso"
	@echo "After install, run: make iso && make run-grub"