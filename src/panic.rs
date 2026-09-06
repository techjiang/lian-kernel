//! Panic handler — prints a diagnostic message and halts.

use core::panic::PanicInfo;
use core::fmt::Write;

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    let msg = info.message();
    let loc = info.location();

    crate::serial_println!("\n\n=== LIAN KERNEL PANIC ===");

    if let Some(l) = loc {
        crate::serial_println!("  at {}:{}:{}", l.file(), l.line(), l.column());
    }

    let _ = write!(crate::serial::SerialWriter, "  message: {}\n", msg);

    // Also to VGA.
    crate::println!("\n\n=== LIAN KERNEL PANIC ===");
    let _ = write!(&crate::vga::WRITER, "  message: {}\n", msg);

    crate::serial_println!("  halting CPU.");

    unsafe {
        loop {
            core::arch::asm!("cli; hlt", options(nostack, preserves_flags));
        }
    }
}
