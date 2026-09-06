//! VGA text-mode buffer (0xB8000).
//!
//! Supports colored text output and scrolling.

use core::ptr::NonNull;

/// The standard 80×25 text buffer lives at physical address 0xB8000.
const VGA_BUFFER_ADDR: usize = 0xB8000;

const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

/// Standard VGA color codes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xA,
    LightCyan = 0xB,
    LightRed = 0xC,
    Pink = 0xD,
    Yellow = 0xE,
    White = 0xF,
}

/// A combined foreground+background color code.
#[derive(Debug, Clone, Copy)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(fg: Color, bg: Color) -> Self {
        ColorCode((bg as u8) << 4 | (fg as u8))
    }
}

/// One screen character: byte + color.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenChar {
    char: u8,
    color: ColorCode,
}

/// Static, singleton VGA writer.
pub static WRITER: VgaWriter = VgaWriter::new();

pub struct VgaWriter {
    column: core::sync::atomic::AtomicUsize,
    row_color: core::sync::atomic::AtomicU8,
}

impl VgaWriter {
    pub const fn new() -> Self {
        VgaWriter {
            column: core::sync::atomic::AtomicUsize::new(0),
            row_color: core::sync::atomic::AtomicU8::new(
                ColorCode::new(Color::LightGreen, Color::Black).0,
            ),
        }
    }

    /// Pointer to the hardware VGA buffer.
    fn buffer(&self) -> &'static mut [[ScreenChar; VGA_WIDTH]; VGA_HEIGHT] {
        let ptr = VGA_BUFFER_ADDR as *mut [[ScreenChar; VGA_WIDTH]; VGA_HEIGHT];
        unsafe { &mut *ptr }
    }

    pub fn set_color(&self, fg: Color, bg: Color) {
        self.row_color.store(ColorCode::new(fg, bg).0, core::sync::atomic::Ordering::Relaxed);
    }

    /// Clear the entire screen.
    pub fn clear(&self) {
        let blank = ScreenChar {
            char: b' ',
            color: ColorCode(self.row_color.load(core::sync::atomic::Ordering::Relaxed)),
        };
        let buf = self.buffer();
        for row in buf.iter_mut() {
            for cell in row.iter_mut() {
                *cell = blank;
            }
        }
        self.column.store(0, core::sync::atomic::Ordering::Relaxed);
    }

    /// Write a single byte, interpreting `\n` as newline.
    pub fn write_byte(&self, byte: u8) {
        let color = ColorCode(self.row_color.load(core::sync::atomic::Ordering::Relaxed));
        match byte {
            b'\n' => self.newline(),
            b'\r' => self.column.store(0, core::sync::atomic::Ordering::Relaxed),
            // Printable ASCII
            0x20..=0x7E => {
                let mut col = self.column.load(core::sync::atomic::Ordering::Relaxed);
                if col >= VGA_WIDTH {
                    self.newline();
                    col = 0;
                }
                let row = 0; // will be replaced with a proper row tracker
                // Actually we need a row variable; let's store row in column high bits
                // Redo: store row+col in a single usize
                let _ = row;
                // Simplified: use a static atomic for row
                let r = ROW.load(core::sync::atomic::Ordering::Relaxed);
                let buf = self.buffer();
                buf[r][col] = ScreenChar { char: byte, color };
                col += 1;
                self.column.store(col, core::sync::atomic::Ordering::Relaxed);
            }
            _ => {
                // Non-ASCII: print a diamond
                self.write_byte(0x04);
            }
        }
    }

    fn newline(&self) {
        let r = ROW.load(core::sync::atomic::Ordering::Relaxed);
        if r >= VGA_HEIGHT - 1 {
            self.scroll_up();
        } else {
            ROW.store(r + 1, core::sync::atomic::Ordering::Relaxed);
        }
        self.column.store(0, core::sync::atomic::Ordering::Relaxed);
    }

    fn scroll_up(&self) {
        let buf = self.buffer();
        // Shift all rows up by 1.
        for r in 1..VGA_HEIGHT {
            let src = buf[r];
            buf[r - 1] = src;
        }
        // Clear last row.
        let blank = ScreenChar {
            char: b' ',
            color: ColorCode(self.row_color.load(core::sync::atomic::Ordering::Relaxed)),
        };
        for cell in buf[VGA_HEIGHT - 1].iter_mut() {
            *cell = blank;
        }
    }

    /// Write a string slice (ASCII only; non-ASCII becomes diamond).
    pub fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

// We use a separate static for the row because `AtomicUsize` for column alone
// is insufficient.
static ROW: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

// ─── core::fmt support ───────────────────────────────────────────────────────

use core::fmt;

impl fmt::Write for &VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        VgaWriter::write_str(self, s);
        Ok(())
    }
}

/// Print a formatted string to the VGA buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!(&crate::vga::WRITER, $($arg)*);
    });
}

/// Like `print!` with a trailing newline.
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => ({
        $crate::print!("{}\n", format_args!($($arg)*));
    });
}
