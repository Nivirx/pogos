# pogOS
pogOS is a toyOS/Kernel for x86_64 processors.

# Building
pogOS requires an up to date nightly rustc (tested with 1.28.2 as of 6-17-18) and xargo
non-rust dependencies include autotools, NASM, GCC, and grub-mkrescue and xorriso (to generate a multiboot image)
all rust dependencies will be handled by cargo

to build enter into the root project directory and run 'make && make iso'
to quickly run pogOS issue the command 'make run' (qemu x86-64 is required)
