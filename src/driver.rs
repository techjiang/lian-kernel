//! Driver framework.
//!
//! Drivers in the Lian microkernel architecture are (eventually) user-space
//! tasks.  For now, the essential platform drivers (timer, keyboard) live
//! in the kernel and are registered here.

/// Initialize all platform drivers.
pub fn init() {
    crate::arch::x86_64::timer::init();
    crate::println!("[driver] Timer driver loaded");

    // Unmask timer IRQ (IRQ 0) and keyboard IRQ (IRQ 1).
    unsafe {
        crate::arch::x86_64::pic::unmask(0);
        crate::arch::x86_64::pic::unmask(1);
    }
    crate::println!("[driver] Keyboard driver loaded (IRQ1)");
}
