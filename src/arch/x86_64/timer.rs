//! Programmable Interval Timer (PIT) — the classic 8254 timer at I/O 0x40.
//!
//! Channel 0 is connected to IRQ0 and ticks at a configurable rate.

use crate::arch::x86_64::port::{outb, inb, io_wait};

const PIT_CHANNEL0: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

/// PIT base frequency: 1193182 Hz.
const PIT_BASE_FREQ: u32 = 1_193_182;

/// Target frequency: 100 Hz (one tick every 10 ms).
const TARGET_FREQ: u32 = 100;

/// Tick counter — incremented on each timer interrupt.
static TICKS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// Initialize the PIT to fire at `TARGET_FREQ`.
pub fn init() {
    let divisor = PIT_BASE_FREQ / TARGET_FREQ;
    let lo = (divisor & 0xFF) as u8;
    let hi = ((divisor >> 8) & 0xFF) as u8;

    unsafe {
        // Command: channel 0, access lo+hi, mode 2 (rate generator), binary.
        outb(PIT_COMMAND, 0x36);
        io_wait();
        outb(PIT_CHANNEL0, lo);
        io_wait();
        outb(PIT_CHANNEL0, hi);
        io_wait();
    }

    crate::println!("[timer] PIT initialized at {} Hz", TARGET_FREQ);
}

/// Called from the IRQ0 handler.
pub fn on_tick() {
    let count = TICKS.fetch_add(1, core::sync::atomic::Ordering::Relaxed) + 1;

    // Every second (100 ticks), print a heartbeat.
    if count % 100 == 0 {
        crate::serial_println!("[timer] tick #{} ({} s)", count, count / 100);
    }

    // Notify the scheduler.
    crate::process::scheduler::on_timer_tick();
}

/// Return the total number of ticks since boot.
pub fn ticks() -> u64 {
    TICKS.load(core::sync::atomic::Ordering::Relaxed)
}
