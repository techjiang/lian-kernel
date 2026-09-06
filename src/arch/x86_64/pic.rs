//! Programmable Interrupt Controller (8259 PIC) driver.
//!
//! The legacy 8259 PIC maps IRQs 0–15 to CPU vectors 0x08–0x0F by default,
//! which conflicts with CPU exceptions.  We remap IRQs to vectors 0x20–0x2F.

use crate::arch::x86_64::port::{outb, inb, io_wait};

// ─── PIC I/O ports ──────────────────────────────────────────────────────────
const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const ICW1_ICW4: u8 = 0x11;   // ICW4 needed + cascade mode
const ICW1_SINGLE: u8 = 0x02; // single mode (no)
const ICW4_8086: u8 = 0x01;  // 8086 mode

/// Offset for PIC1 (IRQ 0 → vector 0x20).
pub const PIC1_OFFSET: u8 = 0x20;
/// Offset for PIC2 (IRQ 8 → vector 0x28).
pub const PIC2_OFFSET: u8 = 0x28;

/// End-of-Interrupt command.
const EOI: u8 = 0x20;

/// Initialize and remap the 8259 PICs.
///
/// # Safety
/// Must be called once during boot, with interrupts disabled.
pub unsafe fn init() {
    // ICW1: start init, cascade, expect ICW4.
    outb(PIC1_CMD, ICW1_ICW4);
    io_wait();
    outb(PIC2_CMD, ICW1_ICW4);
    io_wait();

    // ICW2: vector offsets.
    outb(PIC1_DATA, PIC1_OFFSET);
    io_wait();
    outb(PIC2_DATA, PIC2_OFFSET);
    io_wait();

    // ICW3: tell PIC1 about the slave on IRQ2, tell PIC2 its cascade identity.
    outb(PIC1_DATA, 0x04); // bit 2 = slave on IRQ2
    io_wait();
    outb(PIC2_DATA, 0x02); // slave cascade identity = 2
    io_wait();

    // ICW4: 8086 mode.
    outb(PIC1_DATA, ICW4_8086);
    io_wait();
    outb(PIC2_DATA, ICW4_8086);
    io_wait();

    // Mask all interrupts initially; specific IRQs are unmasked by their drivers.
    outb(PIC1_DATA, 0xFF);
    outb(PIC2_DATA, 0xFF);
}

/// Send EOI for the given IRQ number (0–15).
///
/// # Safety
/// Must only be called from an interrupt handler context.
pub unsafe fn end_of_interrupt(irq: u8) {
    if irq >= 8 {
        outb(PIC2_CMD, EOI);
    }
    outb(PIC1_CMD, EOI);
}

/// Mask an IRQ (disable it).
pub unsafe fn mask(irq: u8) {
    let (port, bit) = if irq < 8 {
        (PIC1_DATA, irq)
    } else {
        (PIC2_DATA, irq - 8)
    };
    let value = inb(port) | (1 << bit);
    outb(port, value);
}

/// Unmask an IRQ (enable it).
pub unsafe fn unmask(irq: u8) {
    let (port, bit) = if irq < 8 {
        (PIC1_DATA, irq)
    } else {
        (PIC2_DATA, irq - 8)
    };
    let value = inb(port) & !(1 << bit);
    outb(port, value);
}
