//! Lian — a top-tier microkernel written in Rust from scratch.
//!
//! ## Architecture
//!
//! Lian follows a microkernel design:
//!   • The kernel core provides scheduling, IPC, and address-space management.
//!   • Device drivers and filesystem services run as separate tasks.
//!   • Communication is via message-passing IPC.
//!
//! ## Current target
//!
//! x86_64 (bare metal, Multiboot2 boot).  The HAL in `hal.rs` defines the
//! traits for multi-architecture support (ARM64, RISC-V planned).

#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

// ─── External crates ────────────────────────────────────────────────────────
extern crate alloc;

// ─── Global allocator ────────────────────────────────────────────────────────
#[global_allocator]
static ALLOCATOR: memory::heap::LockedHeap = memory::heap::LockedHeap;

// ─── Modules ─────────────────────────────────────────────────────────────────
pub mod util;
pub mod sync;
pub mod arch;
pub mod hal;
pub mod vga;
pub mod serial;
pub mod panic;
pub mod memory;
pub mod process;
pub mod syscall;
pub mod fs;
pub mod ipc;
pub mod driver;

// ─── Re-exports ──────────────────────────────────────────────────────────────
// The `print!`, `println!`, `serial_print!`, `serial_println!` macros are
// exported at the crate root via `#[macro_export]` in vga.rs / serial.rs.

// ─── IRQ dispatch ────────────────────────────────────────────────────────────
/// Route a hardware IRQ (from the PIC) to the appropriate driver.
pub fn irq_dispatch(irq: u8) {
    match irq {
        0 => arch::x86_64::timer::on_tick(),
        1 => arch::x86_64::keyboard::on_interrupt(),
        _ => {
            // Unhandled IRQ — just log it.
            if irq < 16 {
                serial_println!("[irq] unhandled IRQ {}", irq);
            }
        }
    }
}
