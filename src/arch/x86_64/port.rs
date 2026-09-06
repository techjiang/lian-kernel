//! Port I/O primitives for x86_64.
//!
//! These are the lowest-level building blocks for talking to hardware.

use core::arch::asm;

/// Write a byte to an I/O port.
#[inline]
pub unsafe fn outb(port: u16, val: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") val,
        options(nostack, preserves_flags),
    );
}

/// Read a byte from an I/O port.
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    asm!(
        "in al, dx",
        out("al") ret,
        in("dx") port,
        options(nostack, preserves_flags),
    );
    ret
}

/// Write a word (16-bit) to an I/O port.
#[inline]
pub unsafe fn outw(port: u16, val: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") val,
        options(nostack, preserves_flags),
    );
}

/// Read a word (16-bit) from an I/O port.
#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let ret: u16;
    asm!(
        "in ax, dx",
        out("ax") ret,
        in("dx") port,
        options(nostack, preserves_flags),
    );
    ret
}

/// Write a dword (32-bit) to an I/O port.
#[inline]
pub unsafe fn outl(port: u16, val: u32) {
    asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") val,
        options(nostack, preserves_flags),
    );
}

/// Read a dword (32-bit) from an I/O port.
#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let ret: u32;
    asm!(
        "in eax, dx",
        out("eax") ret,
        in("dx") port,
        options(nostack, preserves_flags),
    );
    ret
}

/// I/O wait — writes a dummy byte to port 0x80 to create a small delay.
#[inline]
pub fn io_wait() {
    unsafe { outb(0x80, 0) };
}
