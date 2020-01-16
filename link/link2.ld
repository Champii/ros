/* src/arch/amd64/linker.ld */
ENTRY(start)

SECTIONS {
  . = 1M;

  .rodata BLOCK(4K) : ALIGN(4K)
  {
    /* ensure that the multiboot header is at the beginning */
    KEEP(*(.multiboot_header))
    *(.rodata .rodata.*)
    . = ALIGN(4K);
  }

  .text BLOCK(4K) : ALIGN(4K)
  {
    *(.text .text.*)
    . = ALIGN(4K);
  }

  .data BLOCK(4K) : ALIGN(4K)
  {
    *(.data .data.*)
    . = ALIGN(4K);
  }

  .bss BLOCK(4K) : ALIGN(4K)
  {
    *(.bss .bss.*)
    . = ALIGN(4K);
  }

  .got BLOCK(4K) : ALIGN(4K)
  {
    *(.got)
    . = ALIGN(4K);
  }

  .got.plt BLOCK(4K) : ALIGN(4K)
  {
    *(.got.plt)
    . = ALIGN(4K);
  }

  .data.rel.ro : ALIGN(4K) {
    *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    . = ALIGN(4K);
  }

  .gcc_except_table : ALIGN(4K) {
    *(.gcc_except_table)
    . = ALIGN(4K);
  }


  /DISCARD/ :
  {
      *(.*)
      *(.symtab)
      *(.strtab)
      *(.strtab)
      *(.shstrtab)
  }

}