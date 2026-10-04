/* Keraunos x86-64 kernel linker script.
 *
 * The kernel is linked at a 1 MiB identity-mapped physical address, which is
 * where GRUB2/multiboot2 loads ELF64 images before entering long mode.
 */

OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

SECTIONS
{
    . = 1M;

    .text : ALIGN(4K)
    {
        KEEP(*(.multiboot2_header))
        *(.text.entry)
        *(.text .text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }

    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss .bss.* COMMON)
        __bss_end = .;
    }

    /* Boot stack (not zeroed by the entry stub). */
    .stack (NOLOAD) : ALIGN(16)
    {
        _stack_bottom = .;
        . += 64K;
        _stack_top = .;
    }

    _kernel_end = ALIGN(4K);

    /DISCARD/ :
    {
        *(.eh_frame*)
        *(.note .note.*)
        *(.comment)
    }
}
