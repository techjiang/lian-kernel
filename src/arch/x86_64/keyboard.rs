//! PS/2 keyboard driver (scancode Set 1).
//!
//! IRQ1 → vector 0x21 (PIC offset 0x20 + 1).

use crate::arch::x86_64::port::{inb, outb};

const KEYBOARD_DATA: u16 = 0x60;
const KEYBOARD_STATUS: u16 = 0x64;
const KEYBOARD_COMMAND: u16 = 0x64;

/// Status register bits.
const OUTPUT_BUFFER_FULL: u8 = 0x01;
const INPUT_BUFFER_FULL: u8 = 0x02;

/// Scancode Set 1 → ASCII (US layout, simplified).
const SCANCODE_MAP: [u8; 128] = {
    let mut map = [0u8; 128];
    map[0x01] = b'\x1b'; // Esc
    map[0x02] = b'1';
    map[0x03] = b'2';
    map[0x04] = b'3';
    map[0x05] = b'4';
    map[0x06] = b'5';
    map[0x07] = b'6';
    map[0x08] = b'7';
    map[0x09] = b'8';
    map[0x0A] = b'9';
    map[0x0B] = b'0';
    map[0x0C] = b'-';
    map[0x0D] = b'=';
    map[0x0E] = b'\x08'; // Backspace
    map[0x0F] = b'\t';   // Tab
    map[0x10] = b'q';
    map[0x11] = b'w';
    map[0x12] = b'e';
    map[0x13] = b'r';
    map[0x14] = b't';
    map[0x15] = b'y';
    map[0x16] = b'u';
    map[0x17] = b'i';
    map[0x18] = b'o';
    map[0x19] = b'p';
    map[0x1A] = b'[';
    map[0x1B] = b']';
    map[0x1C] = b'\n';   // Enter
    map[0x1D] = 0;       // Left Ctrl
    map[0x1E] = b'a';
    map[0x1F] = b's';
    map[0x20] = b'd';
    map[0x21] = b'f';
    map[0x22] = b'g';
    map[0x23] = b'h';
    map[0x24] = b'j';
    map[0x25] = b'k';
    map[0x26] = b'l';
    map[0x27] = b';';
    map[0x28] = b'\'';
    map[0x29] = b'`';
    map[0x2A] = 0;       // Left Shift
    map[0x2B] = b'\\';
    map[0x2C] = b'z';
    map[0x2D] = b'x';
    map[0x2E] = b'c';
    map[0x2F] = b'v';
    map[0x30] = b'b';
    map[0x31] = b'n';
    map[0x32] = b'm';
    map[0x33] = b',';
    map[0x34] = b'.';
    map[0x35] = b'/';
    map[0x36] = 0;       // Right Shift
    map[0x37] = b'*';    // Keypad *
    map[0x38] = 0;       // Left Alt
    map[0x39] = b' ';    // Space
    map[0x3A] = 0;       // Caps Lock
    map
};

/// Shifted scancode map (when Shift is held).
const SCANCODE_MAP_SHIFT: [u8; 128] = {
    let mut map = [0u8; 128];
    map[0x02] = b'!';
    map[0x03] = b'@';
    map[0x04] = b'#';
    map[0x05] = b'$';
    map[0x06] = b'%';
    map[0x07] = b'^';
    map[0x08] = b'&';
    map[0x09] = b'*';
    map[0x0A] = b'(';
    map[0x0B] = b')';
    map[0x0C] = b'_';
    map[0x0D] = b'+';
    map[0x10] = b'Q';
    map[0x11] = b'W';
    map[0x12] = b'E';
    map[0x13] = b'R';
    map[0x14] = b'T';
    map[0x15] = b'Y';
    map[0x16] = b'U';
    map[0x17] = b'I';
    map[0x18] = b'O';
    map[0x19] = b'P';
    map[0x1A] = b'{';
    map[0x1B] = b'}';
    map[0x1E] = b'A';
    map[0x1F] = b'S';
    map[0x20] = b'D';
    map[0x21] = b'F';
    map[0x22] = b'G';
    map[0x23] = b'H';
    map[0x24] = b'J';
    map[0x25] = b'K';
    map[0x26] = b'L';
    map[0x27] = b':';
    map[0x28] = b'"';
    map[0x29] = b'~';
    map[0x2B] = b'|';
    map[0x2C] = b'Z';
    map[0x2D] = b'X';
    map[0x2E] = b'C';
    map[0x2F] = b'V';
    map[0x30] = b'B';
    map[0x31] = b'N';
    map[0x32] = b'M';
    map[0x33] = b'<';
    map[0x34] = b'>';
    map[0x35] = b'?';
    map
};

static SHIFT_HELD: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Ring buffer for key events.
const KEYBUF_SIZE: usize = 256;

struct KeyBuffer {
    buf: [u8; KEYBUF_SIZE],
    head: usize,
    tail: usize,
}

impl KeyBuffer {
    const fn new() -> Self {
        KeyBuffer {
            buf: [0; KEYBUF_SIZE],
            head: 0,
            tail: 0,
        }
    }

    fn push(&mut self, byte: u8) {
        let next = (self.tail + 1) % KEYBUF_SIZE;
        if next != self.head {
            self.buf[self.tail] = byte;
            self.tail = next;
        }
        // else: buffer full, drop the key.
    }

    fn pop(&mut self) -> Option<u8> {
        if self.head == self.tail {
            None
        } else {
            let byte = self.buf[self.head];
            self.head = (self.head + 1) % KEYBUF_SIZE;
            Some(byte)
        }
    }
}

static KEYBUF: crate::sync::spinlock::SpinLock<KeyBuffer> =
    crate::sync::spinlock::SpinLock::new(KeyBuffer::new());

/// Called from the IRQ1 handler.
pub fn on_interrupt() {
    let scancode = unsafe { inb(KEYBOARD_DATA) };

    // Key release: bit 7 set in Set 1.
    let released = scancode & 0x80 != 0;
    let code = scancode & 0x7F;

    // Handle modifier keys.
    if code == 0x2A || code == 0x36 {
        // Left/Right Shift
        SHIFT_HELD.store(!released, core::sync::atomic::Ordering::Relaxed);
        return;
    }

    if released {
        return; // ignore key releases
    }

    // Extended scancode prefix (0xE0) — ignore for now.
    if scancode == 0xE0 {
        return;
    }

    // Map to ASCII.
    let ascii = if SHIFT_HELD.load(core::sync::atomic::Ordering::Relaxed) {
        SCANCODE_MAP_SHIFT.get(code as usize).copied().unwrap_or(0)
    } else {
        SCANCODE_MAP.get(code as usize).copied().unwrap_or(0)
    };

    if ascii != 0 {
        let mut buf = KEYBUF.lock();
        buf.push(ascii);
    }
}

/// Read the next key (non-blocking).
pub fn read_key() -> Option<u8> {
    KEYBUF.lock().pop()
}

/// Wait for a key press (blocking).
pub fn wait_for_key() -> u8 {
    loop {
        if let Some(k) = read_key() {
            return k;
        }
        // Halt until next interrupt.
        unsafe { core::arch::asm!("hlt", options(nostack, preserves_flags)) };
    }
}
