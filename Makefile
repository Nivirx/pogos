arch ?= x86_64
target ?= $(arch)-unknown-none

rust_os := target/$(target)/debug/libpog_os.a
rust_os_release := target/$(target)/release/libpog_os.a

kernel := out/kernel-$(arch).bin
kernel-release := out/kernel-release-$(arch).bin

iso := out/pogOS-$(arch).iso
iso-release := out/pogOS-release-$(arch).iso

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
asm_source_files := $(wildcard src/arch/$(arch)/*asm)
asm_object_files := $(patsubst src/arch/$(arch)/%.asm, build/arch/$(arch)/%.o, $(asm_source_files))

out_dir := out/
build_dir := build/

qemu_args := -serial stdio -d int -no-shutdown -no-reboot  -m 512M -sdl
.PHONY: all release debug nasm_stage clean run iso iso-release kernel kernel-release 

all: $(kernel) $(kernel-release)
release: $(kernel-release)
debug: $(kernel)
nasm_stage: $(asm_object_files)

clean:
	@rm -r $(out_dir)
	@rm -r $(build_dir)
	@RUST_TARGET_PATH=$(shell pwd) cargo clean --target ./$(target).json

run: $(iso)
	@qemu-system-$(arch) $(qemu_args) -enable-kvm -cpu host -cdrom $(iso)
run-sb: $(iso)
	@qemu-system-$(arch) $(qemu_args) -cpu SandyBridge -cdrom $(iso)
run-epyc: $(iso)
	@qemu-system-$(arch) $(qemu_args) -cpu EPYC -cdrom $(iso)
iso: $(iso)
iso-release: $(iso-release)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub/grub.cfg
	@grub-mkrescue -o $(iso) -d /usr/lib/grub/i386-pc build/isofiles/ 2> /dev/null
	@rm -r build/isofiles

$(iso-release): $(kernel-release) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel-release) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub/grub.cfg
	@grub-mkrescue -o $(iso-release) -d /usr/lib/grub/i386-pc build/isofiles/ 2> /dev/null
	@rm -r build/isofiles


$(kernel): kernel $(rust_os) $(asm_object_files) $(linker_script)
	@ld -n --gc-sections -T $(linker_script) -o $(kernel) $(asm_object_files) $(rust_os)

$(kernel-release): kernel-release $(rust_os_release) $(asm_object_files) $(linker_script)
	@ld -n --gc-sections -T $(linker_script) -o $(kernel-release) $(asm_object_files) $(rust_os_release)

kernel:
	@RUST_TARGET_PATH=$(shell pwd) cargo +nightly build -Z build-std --target x86_64-unknown-none --verbose
kernel-release:
	@RUST_TARGET_PATH=$(shell pwd) cargo +nightly build -Z build-std --target x86_64-unknown-none --release

build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@
