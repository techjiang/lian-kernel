//! Interrupt Descriptor Table (IDT) for x86_64.
//!
//! Each entry is a 16-byte "gate descriptor".  When an interrupt/exception
//! fires, the CPU pushes the register context and jumps to the handler
//! through the gate.

use core::arch::asm;
use core::mem;

/// A 16-byte interrupt gate descriptor (64-bit).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    pub offset_low: u16,
    pub selector: u16,
    pub ist: u8,       // bits 0–2 = IST index, rest 0
    pub ty_attr: u8,   // type + attributes
    pub offset_middle: u16,
    pub offset_high: u32,
    pub zero: u32,
}

impl IdtEntry {
    pub const fn missing() -> Self {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            ist: 0,
            ty_attr: 0,
            offset_middle: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    /// Set this entry to point at `handler_addr` with the given IST index.
    pub fn set_handler(&mut self, handler_addr: u64, ist: u8) {
        self.offset_low = (handler_addr & 0xFFFF) as u16;
        self.offset_middle = ((handler_addr >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler_addr >> 32) & 0xFFFFFFFF) as u32;
        self.selector = super::gdt::KERNEL_CODE_SELECTOR;
        self.ist = ist & 0x7;
        // 0x8E = present | ring0 | interrupt gate (64-bit)
        self.ty_attr = 0x8E;
        self.zero = 0;
    }
}

/// IDTR register.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IdtPointer {
    limit: u16,
    base: u64,
}

/// Max 256 vectors.
pub const IDT_ENTRIES: usize = 256;

/// The one and only IDT.
static mut IDT: [IdtEntry; IDT_ENTRIES] = [IdtEntry::missing(); IDT_ENTRIES];

/// Install the IDT.
///
/// # Safety
/// All `set_handler` calls must be done first.
pub unsafe fn load() {
    let idtr = unsafe {
        IdtPointer {
            limit: (IDT_ENTRIES * mem::size_of::<IdtEntry>() - 1) as u16,
            base: core::ptr::addr_of!(IDT) as u64,
        }
    };
    asm!("lidt [{}]", in(reg) &idtr, options(nostack, preserves_flags));
}

/// Set a handler for vector `n` (0-indexed).
///
/// # Safety
/// Must be called before `load()`.
pub unsafe fn set_handler(n: usize, handler_addr: u64, ist: u8) {
    unsafe {
        IDT[n].set_handler(handler_addr, ist);
    }
}

// ─── CPU context saved on the stack ─────────────────────────────────────────

/// The CPU register context saved by the interrupt stub.
///
/// The layout **must match** the `push` / `sub rsp` sequence in
/// the assembly stubs defined in `interrupts.rs`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InterruptFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    // The CPU pushes these automatically:
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl InterruptFrame {
    /// Print the frame for debugging.
    pub fn dump(&self) {
        crate::println!("--- Interrupt Frame ---");
        crate::println!("  RAX={:#018x}  RBX={:#018x}  RCX={:#018x}", self.rax, self.rbx, self.rcx);
        crate::println!("  RDX={:#018x}  RSI={:#018x}  RDI={:#018x}", self.rdx, self.rsi, self.rdi);
        crate::println!("  RBP={:#018x}  R8 ={:#018x}  R9 ={:#018x}", self.rbp, self.r8, self.r9);
        crate::println!("  R10={:#018x}  R11={:#018x}  R12={:#018x}", self.r10, self.r11, self.r12);
        crate::println!("  R13={:#018x}  R14={:#018x}  R15={:#018x}", self.r13, self.r14, self.r15);
        crate::println!("  RIP={:#018x}  CS ={:#018x}  FLG={:#018x}", self.rip, self.cs, self.rflags);
        crate::println!("  RSP={:#018x}  SS ={:#018x}", self.rsp, self.ss);
    }
}
