# Lian Kernel — Build System
# Targets: build, run, test, clean, iso, doc

KERNEL_TARGET  := x86_64-unknown-none
KERNEL_BINARY := target/$(KERNEL_TARGET)/debug/lian-kernel
KERNEL_RELEASE := target/$(KERNEL_TARGET)/release/lian-kernel
ISO_FILE       := target/lian.iso

# QEMU
QEMU           := qemu-system-x86_64
QEMU_FLAGS     := -drive format=raw,file=$(ISO_FILE) -m 256M -serial stdio -no-reboot -no-shutdown
QEMU_DEBUG     := -d int -D target/qemu_log.txt

# GRUB / xorriso
GRUB_MODDIR    := /usr/lib/grub/i386-pc
GRUB_FILE      := grub.cfg

# Rust
CARGO          := cargo

# ─── Targets ──────────────────────────────────────────────────────────────────

.PHONY: all build run test clean iso doc release help

all: build

build:
	$(CARGO) build --target $(KERNEL_TARGET)

release:
	$(CARGO) build --target $(KERNEL_TARGET) --release

iso: build
	@mkdir -p target/isodir/boot/grub
	cp $(KERNEL_BINARY) target/isodir/boot/lian-kernel
	@echo 'menuentry "Lian Kernel" {' > target/isodir/boot/grub/grub.cfg
	@echo '  multiboot2 /boot/lian-kernel' >> target/isodir/boot/grub/grub.cfg
	@echo '}' >> target/isodir/boot/grub/grub.cfg
	grub-mkrescue -o $(ISO_FILE) target/isodir 2>/dev/null || \
	xorriso -as mkrescue -o $(ISO_FILE) target/isodir 2>/dev/null || \
	echo "Warning: grub-mkrescue/xorriso not found. Install with: apt install grub-pc-bin xorriso"

run: iso
	$(QEMU) $(QEMU_FLAGS) || \
	echo "Warning: QEMU not found. Install with: winget install qemu.qemu"

run-direct: build
	$(QEMU) -kernel $(KERNEL_BINARY) -m 256M -serial stdio -no-reboot || \
	echo "Warning: QEMU not found."

test: build
	@echo "Running kernel tests in QEMU..."
	$(QEMU) -kernel $(KERNEL_BINARY) -m 256M -serial stdio -no-reboot -display none \
		-serial file:target/test_output.txt 2>/dev/null || true
	@echo "Test output saved to target/test_output.txt"

clean:
	$(CARGO) clean
	rm -rf target/isodir target/lian.iso target/test_output.txt

doc:
	$(CARGO) doc --target $(KERNEL_TARGET) --no-deps

# ─── Debug ───────────────────────────────────────────────────────────────────

.PHONY: gdb run-gdb

run-gdb: iso
	$(QEMU) $(QEMU_FLAGS) -s -S &
	@echo "QEMU started in debug mode. Connect with: gdb -ex 'target remote localhost:1234'"

gdb:
	@echo "Run: gdb -ex 'target remote localhost:1234' $(KERNEL_BINARY)"

# ─── Help ────────────────────────────────────────────────────────────────────

help:
	@echo "Lian Kernel Build System"
	@echo "======================="
	@echo "  make build    - Compile the kernel (debug)"
	@echo "  make release   - Compile the kernel (release/optimized)"
	@echo "  make iso      - Build bootable ISO image"
	@echo "  make run      - Boot kernel in QEMU (via ISO)"
	@echo "  make run-direct - Boot kernel in QEMU (direct multiboot)"
	@echo "  make test     - Run kernel tests"
	@echo "  make clean    - Remove all build artifacts"
	@echo "  make doc      - Generate documentation"
	@echo "  make run-gdb  - Start QEMU in GDB debug mode"
