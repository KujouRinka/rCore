OBJCOPY := rust-objcopy --binary-architecture=riscv64
KERNEL_ELF := target/riscv64gc-unknown-none-elf/release/os
KERNEL_BIN := target/riscv64gc-unknown-none-elf/release/os.bin
SBI_PATH := bootloader/rustsbi-qemu.bin
DEVICE_PARAM := -device loader,file=$(KERNEL_BIN),addr=0x80200000
SOURCES := $(shell find src -name '*')

$(KERNEL_BIN): $(KERNEL_ELF)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

$(KERNEL_ELF): $(SOURCES)
	@cargo build --release

run: $(KERNEL_BIN)
	qemu-system-riscv64 -machine virt -bios $(SBI_PATH) -nographic $(DEVICE_PARAM)

run-gdb: $(KERNEL_BIN)
	qemu-system-riscv64 -machine virt -bios $(SBI_PATH) -nographic $(DEVICE_PARAM) -s -S