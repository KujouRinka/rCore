OBJCOPY := rust-objcopy --binary-architecture=riscv64
KERNEL_ELF := target/riscv64gc-unknown-none-elf/release/os
KERNEL_BIN := target/riscv64gc-unknown-none-elf/release/os.bin
SBI_PATH := bootloader/rustsbi-qemu.bin
DEVICE_PARAM := -device loader,file=$(KERNEL_BIN),addr=0x80200000
SOURCES := $(shell find src -name '*')
# USER_SOURCES := $(shell find ../user/target/riscv64gc-unknown-none-elf/release -name '*.bin')
USER_SOURCES := $(shell find ../user/src -name '*')

$(KERNEL_BIN): $(KERNEL_ELF)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

$(KERNEL_ELF): $(SOURCES) $(USER_SOURCES)
	# make build for ../user dir and switch back dir
	cd ../user && make build && cd ../os
	@cargo build --release

run: $(KERNEL_BIN)
	qemu-system-riscv64 -machine virt -m 128M -bios $(SBI_PATH) -nographic $(DEVICE_PARAM)

run-gdb: $(KERNEL_BIN)
	qemu-system-riscv64 -machine virt -m 128M -bios $(SBI_PATH) -nographic $(DEVICE_PARAM) -s -S

clean:
	cargo clean