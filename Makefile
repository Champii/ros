default:
	cargo xbuild
	nasm -f elf64 asm/multiboot.S
	nasm -f elf64 asm/boot.S  
	nasm -f elf64 asm/long_mode_init.S 
	ld -n -T link/link2.ld -o build/isofiles/boot/kernel.bin asm/boot.o asm/multiboot.o asm/long_mode_init.o target/x86_64-ros/debug/libros.a
	grub-mkrescue -o build/os.iso build/isofiles 
	qemu-system-x86_64 -cdrom build/os.iso -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio