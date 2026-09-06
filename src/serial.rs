//! Serial port output (COM1).
//!
//! On x86 the 16550 UART at I/O port 0x3F8 is the canonical debug channel.
//! GRUB and QEMU both mirror serial output to the host terminal.

use core::fmt;

/// COM1 I/O base.
const COM1: u16 = 0x3F8;

/// 16550 register offsets.
const THR: u16 = 0; // Transmit Holding Register
const RBR: u16 = 0; // Receive Buffer Register
const IER: u16 = 1; // Interrupt Enable Register
const FCR: u16 = 2; // FIFO Control Register
const LCR: u16 = 3; // Line Control Register
const MCR: u16 = 4; // Modem Control Register
const LSR: u16 = 5; // Line Status Register

#[inline]
unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") val,
        options(nostack, preserves_flags),
    );
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    core::arch::asm!(
        "in al, dx",
        out("al") ret,
        in("dx") port,
        options(nostack, preserves_flags),
    );
    ret
}

/// Initialize the 16550 UART on COM1.
pub fn init() {
    unsafe {
        // Disable interrupts.
        outb(COM1 + IER, 0x00);
        // Enable DLAB (Divisor Latch Access Bit) to set baud rate.
        outb(COM1 + LCR, 0x80);
        // 38400 baud: divisor = 3.
        outb(COM1 + 0, 0x03);
        outb(COM1 + 1, 0x00);
        // 8 bits, no parity, 1 stop bit; disable DLAB.
        outb(COM1 + LCR, 0x03);
        // Enable FIFO, clear them, 14-byte threshold.
        outb(COM1 + FCR, 0xC7);
        // RTS/DSR set, OUT2 (aux output 2 — needed for interrupts).
        outb(COM1 + MCR, 0x0B);
    }
}

/// Is the transmit holding register empty?
fn is_transmit_empty() -> bool {
    unsafe { (inb(COM1 + LSR) & 0x20) != 0 }
}

/// Send one byte over the serial line.
pub fn write_byte(byte: u8) {
    // Wait until the UART can accept a byte.
    while !is_transmit_empty() {
        core::hint::spin_loop();
    }
    unsafe { outb(COM1 + THR, byte) };
}

/// Print a raw string.
pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

/// Serial writer — implements `core::fmt::Write`.
pub struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Self::write_str_pub(s);
        Ok(())
    }
}

impl SerialWriter {
    pub fn write_str_pub(s: &str) {
        write_str(s);
    }
}

/// Print a formatted string to serial.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        let mut writer = $crate::serial::SerialWriter;
        use core::fmt::Write;
        let _ = write!(writer, $($arg)*);
    });
}

/// Like `serial_print!` with a trailing newline.
#[macro_export]
macro_rules! serial_println {
    () => { $crate::serial_print!("\n") };
    ($($arg:tt)*) => ({
        $crate::serial_print!("{}\n", format_args!($($arg)*));
    });
}
