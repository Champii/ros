MAGIC       equ 0xe85250d6 
MBALIGN     equ  1<<0                   ; align loaded modules on page boundaries
MEMINFO     equ  1<<1
; FLAGS       equ  MBALIGN | MEMINFO
FLAGS       equ  0
CHECKSUM    equ -(MAGIC + FLAGS)
section .multiboot_header
header_start:
    dd MAGIC                ; magic number (multiboot 2)
    dd FLAGS                         ; architecture 0 (protected mode i386)
    dd header_end - header_start ; header length
    ; checksum
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))
    ; dd CHECKSUM

    ; insert optional multiboot tags here

    ; required end tag
    dw 0    ; type
    dw 0    ; flags
    dd 8    ; size
header_end: