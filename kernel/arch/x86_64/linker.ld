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

    /* Boot page tables (PML4 / PDPT / PD), built by the 32-bit entry
       stub. Deliberately outside [__bss_start, __bss_end) so the 64-bit
       continuation's .bss sweep cannot wipe them. */
    .boot_pgtables (NOLOAD) : ALIGN(4K)
    {
        _pml4 = .;
        . += 4K;
        _pdpt = .;
        . += 4K;
        _pd = .;
        . += 4K;
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
