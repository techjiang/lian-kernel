//! Lian Kernel — boot entry point and initialization sequence.
//!
//! The kernel is booted via Multiboot1 (QEMU) or Multiboot2 (GRUB2).
//! The boot assembly stub sets up long mode and calls `kmain`.

#![no_std]
#![no_main]

extern crate alloc;

use core::arch::global_asm;
use lian::*;
use alloc::vec::Vec;

// ─── Multiboot2 header + PVH note + boot stub (32-bit entry → 64-bit long mode) ─

global_asm!(
    r#"
.section .multiboot2, "a"
.align 8
header_start:
    .long 0xE85250D6
    .long 0
    .long header_end - header_start
    .long -(0xE85250D6 + 0 + (header_end - header_start))

    .short 1
    .short 0
    .long 8 + 3*4
    .long 1
    .long 2
    .long 6

    .short 0
    .short 0
    .long 8
header_end:

.section .note.Xen, "a", @note
.align 4
pvh_note:
    .long 4                 # n_namesz: len("Xen\0")
    .long 4                 # n_descsz: 4 bytes (32-bit entry addr)
    .long 0x12              # n_type: XEN_ELFNOTE_PHYS32_ENTRY (18)
    .asciz "Xen"            # n_name: "Xen\0"
    .long _start            # descriptor: 32-bit entry point

.section .bss
.align 16
stack_bottom:
    .skip 65536
stack_top:

.align 4096
pml4_table:
    .skip 4096
pdpt_table:
    .skip 4096
pd_table:
    .skip 4096

.section .rodata
.align 8
boot_gdt:
    .quad 0
    .quad 0x00AF9A000000FFFF
    .quad 0x00CF92000000FFFF
boot_gdt_end:
boot_gdt_ptr:
    .short boot_gdt_end - boot_gdt - 1
    .quad boot_gdt

.section .text
.intel_syntax noprefix
.code32
.globl _start
.type _start, @function
_start:
    mov esp, OFFSET stack_top

    mov eax, OFFSET pdpt_table
    or eax, 3
    mov [pml4_table], eax

    mov eax, OFFSET pd_table
    or eax, 3
    mov [pdpt_table], eax

    mov ecx, 0
pd_fill_loop:
    mov eax, ecx
    shl eax, 21
    or eax, 0x83
    mov [pd_table + ecx*8], eax
    inc ecx
    cmp ecx, 512
    jl pd_fill_loop

    mov eax, cr4
    or eax, 0x20
    mov cr4, eax

    mov eax, OFFSET pml4_table
    mov cr3, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100
    wrmsr

    mov eax, cr0
    or eax, 0x80000001
    mov cr0, eax

    lgdt [boot_gdt_ptr]

    .att_syntax prefix
    ljmp $0x08, $long_mode_start

.code64
.intel_syntax noprefix
long_mode_start:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    xor edi, edi
    xor esi, esi

    call {kmain}

    mov dx, 0x3F8
    mov al, 'D'
    out dx, al
    mov al, 'N'
    out dx, al
    mov al, '\n'
    out dx, al

    cli
2:  hlt
    jmp 2b
"#,
    kmain = sym kmain,
);

// ─── Multiboot constants ─────────────────────────────────────────────────────

const MULTIBOOT1_BOOTLOADER_MAGIC: u32 = 0x2BADB002;
const MULTIBOOT2_BOOTLOADER_MAGIC: u32 = 0x36D76289;

// ─── Memory map parsing ─────────────────────────────────────────────────────

/// Parse Multiboot1 info structure to extract memory regions.
fn parse_multiboot1(mbi_addr: usize) -> Vec<memory::frame::MemoryRegion> {
    let mut regions = Vec::new();

    if mbi_addr == 0 {
        // Fallback if no info provided.
        regions.push(memory::frame::MemoryRegion {
            start: 0,
            len: 1024 * 1024,
            usable: false,
        });
        regions.push(memory::frame::MemoryRegion {
            start: 1024 * 1024,
            len: 32 * 1024 * 1024,
            usable: true,
        });
        return regions;
    }

    let ptr = mbi_addr as *const u32;
    let flags = unsafe { *ptr };

    if flags & 0x1 != 0 {
        // Memory info is available
        let mem_lower = unsafe { *ptr.add(1) } as usize; // in KB
        let mem_upper = unsafe { *ptr.add(2) } as usize; // in KB

        // First 1 MiB: reserved (BIOS, kernel image, etc.)
        regions.push(memory::frame::MemoryRegion {
            start: 0,
            len: 1024 * 1024,
            usable: false,
        });

        // 1 MiB to mem_upper: usable
        let usable_len = mem_upper * 1024;
        regions.push(memory::frame::MemoryRegion {
            start: 1024 * 1024,
            len: usable_len,
            usable: true,
        });
    } else {
        // Fallback.
        regions.push(memory::frame::MemoryRegion {
            start: 1024 * 1024,
            len: 32 * 1024 * 1024,
            usable: true,
        });
    }

    regions
}

/// Parse Multiboot2 info structure to extract memory regions.
fn parse_multiboot2(mbi_addr: usize) -> Vec<memory::frame::MemoryRegion> {
    let mut regions = Vec::new();

    if mbi_addr == 0 {
        regions.push(memory::frame::MemoryRegion {
            start: 0,
            len: 1024 * 1024,
            usable: false,
        });
        regions.push(memory::frame::MemoryRegion {
            start: 1024 * 1024,
            len: 32 * 1024 * 1024,
            usable: true,
        });
        return regions;
    }

    let ptr = mbi_addr as *const u8;
    let total_size = unsafe { *(ptr as *const u32) };

    let mut offset = 8usize;
    while offset + 8 <= total_size as usize {
        let tag_ptr = unsafe { ptr.add(offset) };
        let tag_type = unsafe { *(tag_ptr as *const u32) };
        let tag_size = unsafe { *((tag_ptr as *const u8).add(4) as *const u32) };

        match tag_type {
            6 => {
                let entry_size = unsafe { *((tag_ptr as *const u8).add(8) as *const u32) };
                let entries_start = offset + 16;
                let mut entry_offset = entries_start;

                while entry_offset + entry_size as usize <= offset + tag_size as usize &&
                      entry_offset + 16 <= total_size as usize
                {
                    let base_addr = unsafe {
                        *((ptr.add(entry_offset) as *const u64))
                    };
                    let length = unsafe {
                        *((ptr.add(entry_offset + 8) as *const u64))
                    };
                    let mmap_type = unsafe {
                        *((ptr.add(entry_offset + 16) as *const u32))
                    };

                    regions.push(memory::frame::MemoryRegion {
                        start: base_addr as usize,
                        len: length as usize,
                        usable: mmap_type == 1, // MMAP_AVAILABLE
                    });

                    entry_offset += entry_size as usize;
                }
            }
            0 => break,
            _ => {}
        }

        let next = offset + tag_size as usize;
        offset = (next + 7) & !7;
    }

    if regions.is_empty() {
        regions.push(memory::frame::MemoryRegion {
            start: 1024 * 1024,
            len: 32 * 1024 * 1024,
            usable: true,
        });
    }

    regions
}

// ─── Kernel main ─────────────────────────────────────────────────────────────

/// Kernel C entry point.
///
/// Called from the assembly stub.  The multiboot parameters are currently
/// unused but kept for future memory-map parsing.
#[unsafe(no_mangle)]
pub extern "C" fn kmain(_mbi_addr: usize, _magic: usize) -> ! {
    serial::init();

    vga::WRITER.set_color(vga::Color::LightGreen, vga::Color::Black);
    vga::WRITER.clear();

    println!();
    println!(" ==============================================");
    println!("  Lian Kernel v0.1.0");
    println!("  Architecture: x86_64 (PVH boot, Long Mode)");
    println!(" ==============================================");
    println!();

    serial_println!("Lian kernel: boot OK");

    loop {
        unsafe { core::arch::asm!("hlt", options(nostack)) };
    }
}
