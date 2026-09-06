/*
 * Lian Kernel - Linker Script (x86_64, Multiboot2 + PVH)
 */

ENTRY(_start)

PHDRS
{
    text PT_LOAD FLAGS(5);
    note PT_NOTE FLAGS(0);
}

SECTIONS
{
    . = 1M;

    .note : {
        KEEP(*(.note.Xen))
    } :note

    .text :
    {
        KEEP(*(.multiboot2))
        *(.text .text.*)
    } :text

    .rodata :
    {
        *(.rodata .rodata.*)
    } :text

    .data :
    {
        *(.data .data.*)
    } :text

    .bss :
    {
        bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        bss_end = .;
    } :text

    .got :
    {
        *(.got .got.*)
    } :text

    /DISCARD/ : { *(.eh_frame) *(.eh_frame_hdr) *(.note.gnu.property) }
}
