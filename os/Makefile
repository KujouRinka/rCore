CHAN := release
OBJCOPY := rust-objcopy --binary-architecture=riscv64

KERNEL_ELF := target/riscv64gc-unknown-none-elf/$(CHAN)/os
KERNEL_BIN := target/riscv64gc-unknown-none-elf/$(CHAN)/os.bin

KERNEL_ELF_DEBUG := target/riscv64gc-unknown-none-elf/debug/os
KERNEL_BIN_DEBUG := target/riscv64gc-unknown-none-elf/debug/os.bin

SBI_PATH := bootloader/rustsbi-qemu.bin
DEVICE_PARAM := -device loader,file=$(KERNEL_BIN),addr=0x80200000
SOURCES := $(shell find src -name '*')
# USER_SOURCES := $(shell find ../user/target/riscv64gc-unknown-none-elf/release -name '*.bin')
USER_SOURCES := $(shell find ../user/src -name '*')

CPUS := 3

QEMUOPTS := -machine virt -m 128M -bios $(SBI_PATH) -nographic $(DEVICE_PARAM) -smp $(CPUS)

$(KERNEL_BIN): $(KERNEL_ELF)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

$(KERNEL_BIN_DEBUG): $(KERNEL_ELF_DEBUG)
	@$(OBJCOPY) $(KERNEL_ELF_DEBUG) --strip-all -O binary $@
	@rust-objdump -S $(KERNEL_ELF_DEBUG) > kernel.asm

$(KERNEL_ELF): $(SOURCES) $(USER_SOURCES)
	# make build for ../user dir and switch back dir
	cd ../user && make build && cd ../os
	cargo build --release;

$(KERNEL_ELF_DEBUG): $(SOURCES) $(USER_SOURCES)
	# make build for ../user dir and switch back dir
	cd ../user && make build && cd ../os
	cargo build

run: $(KERNEL_BIN)
	qemu-system-riscv64 $(QEMUOPTS)

run-debug: $(KERNEL_BIN_DEBUG)
	qemu-system-riscv64 $(QEMUOPTS)

run-gdb: $(KERNEL_BIN_DEBUG)
	qemu-system-riscv64 $(QEMUOPTS) -s -S

clean:
	cargo clean