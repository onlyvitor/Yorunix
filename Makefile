BUILD_DIR = build

LD = ld
AS = nasm
CARGO = cargo

ARCH := i386
RUST_TARGET := i686-unknown-linux-gnu
RUST_PROFILE_DIR := debug
RUST_LIB = target/$(RUST_TARGET)/$(RUST_PROFILE_DIR)/libyorunix.a

ENTRY_POINT = arch/x86/boot/entry.asm
ENTRY_OBJ = $(BUILD_DIR)/entry.o

GDT_ASM_SRC = arch/x86/asm/gdt.asm
GDT_ASM_OBJ = $(BUILD_DIR)/gdt_asm.o

IDT_ASM_SRC = arch/x86/asm/idt.asm
IDT_ASM_OBJ = $(BUILD_DIR)/idt_asm.o

LINKER_SCRIPT = link.ld

# ISO / GRUB
ISO_DIR = iso
GRUB_CFG = boot/grub.cfg

LINKER_FLAGS = -z noexecstack --gc-sections
ASFLAGS =

ifeq ($(ARCH), x86_64)
  LINKER_FLAGS += -m elf_x86_64
  ASFLAGS += -f elf64
else
  LINKER_FLAGS += -m elf_i386
  ASFLAGS += -f elf32
endif

.PHONY: all clean run check iso run-grub

all: $(BUILD_DIR)/kernel.bin

# Rust core as a staticlib (no_std, panic=abort). Contains:
# arch/ (gdt, idt) + kernel/ (vga, exceptions, support) + kernel_main + panic_handler.
# Robust list: any new .rs file in src/ triggers a rebuild.
RUST_SRCS := $(shell find src -name '*.rs')
$(RUST_LIB): Cargo.toml rust-toolchain.toml $(RUST_SRCS)
	$(CARGO) build --target $(RUST_TARGET)

$(BUILD_DIR)/kernel.bin: $(ENTRY_OBJ) $(GDT_ASM_OBJ) $(IDT_ASM_OBJ) $(RUST_LIB)
	$(LD) $(LINKER_FLAGS) -T $(LINKER_SCRIPT) -o $@ $(ENTRY_OBJ) $(GDT_ASM_OBJ) $(IDT_ASM_OBJ) $(RUST_LIB)

$(ENTRY_OBJ): $(ENTRY_POINT)
	$(AS) $(ASFLAGS) -o $@ $<

$(GDT_ASM_OBJ): $(GDT_ASM_SRC)
	$(AS) $(ASFLAGS) -o $@ $<

$(IDT_ASM_OBJ): $(IDT_ASM_SRC)
	$(AS) $(ASFLAGS) -o $@ $<

check:
	$(CARGO) fmt --all -- --check && $(CARGO) clippy --all-targets -- -D warnings && $(CARGO) clippy --target $(RUST_TARGET) -- -D warnings && $(CARGO) test --all-targets

clean:
	rm -rf $(BUILD_DIR)
	rm -rf $(ISO_DIR)
	$(CARGO) clean

run: $(BUILD_DIR)/kernel.bin
	qemu-system-x86_64 -kernel $<

$(shell mkdir -p $(BUILD_DIR))

.PHONY: prepare-iso grub-install-instructions

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
