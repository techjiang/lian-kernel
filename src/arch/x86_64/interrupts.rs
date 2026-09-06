//! Interrupt service routine stubs and dispatch.
//!
//! Assembly stubs save the register context, push the vector number, and
//! call the Rust `dispatch` function.  On return they restore context and
//! execute `iretq`.

use core::arch::global_asm;
use crate::arch::x86_64::idt::{self, InterruptFrame};

// ─── Assembly stubs ──────────────────────────────────────────────────────────
//
// Each stub is defined with global_asm!.  The pattern:
//   1. push a dummy if the CPU didn't push an error code
//   2. push the vector number
//   3. jump to the common entry
//
// The common entry saves all GPRs, calls `dispatch`, restores, pops the
// vector number + error code, and does `iretq`.

global_asm!(
    r#"
// ─── Common ISR entry ────────────────────────────────────────────────────
.section .text
.code64
.global isr_common
.type isr_common, @function
isr_common:
    // At this point, the stack has: [error_code, vector_num, RIP, CS, RFLAGS, RSP, SS]
    // (error_code and vector_num pushed by the stub)
    // Save all GPRs:
    push rax
    push rbx
    push rcx
    push rdx
    push rbp
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    // Now the stack looks like:
    //   [r15..rax, error_code, vector, RIP, CS, RFLAGS, RSP, SS]
    // The InterruptFrame struct in Rust expects RIP after the GPRs.
    // Move stack pointer to rdi (1st arg) and vector to rsi (2nd arg).
    mov rdi, rsp            // pointer to the saved frame
    // The vector is at offset (15 * 8) + 8 (after error_code)
    // Actually: after 15 pushes (15*8=120 bytes), the next on stack is
    // error_code, then vector.  So vector is at [rsp + 120 + 8].
    mov rsi, [rsp + 128]    // vector number (offset 120 + 8 = 128)

    cld                     // clear DF (required for any string ops in C ABI)
    call {dispatch}

    // Restore all GPRs:
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rbp
    pop rdx
    pop rcx
    pop rbx
    pop rax

    // Pop the error code and vector number.
    add rsp, 16

    iretq
"#,
    dispatch = sym dispatch,
);

// ─── Generate stubs ─────────────────────────────────────────────────────────
// For brevity and correctness, we generate stubs only for the vectors we
// actually use.  Each stub either pushes a dummy error code (if the CPU
// doesn't) or uses the CPU-pushed one, then pushes the vector and jumps
// to isr_common.

global_asm!(
    r#"
.section .text
.code64

.macro MAKE_ISR_NOERR vector:req
.global isr_\vector
.type isr_\vector, @function
isr_\vector:
    push 0              // dummy error code
    push \vector        // vector number
    jmp isr_common
.endm

.macro MAKE_ISR_ERR vector:req
.global isr_\vector
.type isr_\vector, @function
isr_\vector:
    // CPU already pushed an error code
    push \vector        // vector number
    jmp isr_common
.endm

// CPU exceptions (0–31)
MAKE_ISR_NOERR 0       // #DE Divide by Zero
MAKE_ISR_NOERR 1       // #DB Debug
MAKE_ISR_NOERR 2       // NMI
MAKE_ISR_NOERR 3       // #BP Breakpoint
MAKE_ISR_NOERR 4       // #OF Overflow
MAKE_ISR_NOERR 5       // #BR BOUND Range
MAKE_ISR_NOERR 6       // #UD Invalid Opcode
MAKE_ISR_NOERR 7       // #NM Device Not Available
MAKE_ISR_ERR   8       // #DF Double Fault
MAKE_ISR_NOERR 9       // Coprocessor Segment Overrun
MAKE_ISR_ERR   10      // #TS Invalid TSS
MAKE_ISR_ERR   11      // #NP Segment Not Present
MAKE_ISR_ERR   12      // #SS Stack Fault
MAKE_ISR_ERR   13      // #GP General Protection
MAKE_ISR_ERR   14      // #PF Page Fault
MAKE_ISR_NOERR 15      // Reserved
MAKE_ISR_NOERR 16      // #MF x87 FPU Error
MAKE_ISR_ERR   17      // #AC Alignment Check
MAKE_ISR_NOERR 18      // #MC Machine Check
MAKE_ISR_NOERR 19      // #XM SIMD Exception
MAKE_ISR_NOERR 20      // Virtualization
MAKE_ISR_NOERR 21      // Reserved
MAKE_ISR_NOERR 22      // Reserved
MAKE_ISR_NOERR 23      // Reserved
MAKE_ISR_NOERR 24      // Reserved
MAKE_ISR_NOERR 25      // Reserved
MAKE_ISR_NOERR 26      // Reserved
MAKE_ISR_NOERR 27      // Reserved
MAKE_ISR_NOERR 28      // Reserved
MAKE_ISR_NOERR 29      // Reserved
MAKE_ISR_NOERR 30      // Security
MAKE_ISR_NOERR 31      // Reserved

// IRQs (32–47) — remapped PIC vectors
MAKE_ISR_NOERR 32      // IRQ0  Timer
MAKE_ISR_NOERR 33      // IRQ1  Keyboard
MAKE_ISR_NOERR 34      // IRQ2  Cascade
MAKE_ISR_NOERR 35      // IRQ3  COM2
MAKE_ISR_NOERR 36      // IRQ4  COM1
MAKE_ISR_NOERR 37      // IRQ5  LPT2
MAKE_ISR_NOERR 38      // IRQ6  Floppy
MAKE_ISR_NOERR 39      // IRQ7  LPT1
MAKE_ISR_NOERR 40      // IRQ8  RTC
MAKE_ISR_NOERR 41      // IRQ9  PCI
MAKE_ISR_NOERR 42      // IRQ10
MAKE_ISR_NOERR 43      // IRQ11
MAKE_ISR_NOERR 44      // IRQ12 PS/2 Mouse
MAKE_ISR_NOERR 45      // IRQ13 FPU
MAKE_ISR_NOERR 46      // IRQ14 Primary ATA
MAKE_ISR_NOERR 47      // IRQ15 Secondary ATA

// System call (128 = 0x80)
MAKE_ISR_NOERR 128
"#,
);

// ─── Exception name table ───────────────────────────────────────────────────

const EXCEPTION_NAMES: [&str; 32] = [
    "#DE Divide by Zero",
    "#DB Debug",
    "NMI Non-Maskable Interrupt",
    "#BP Breakpoint",
    "#OF Overflow",
    "#BR BOUND Range Exceeded",
    "#UD Invalid Opcode",
    "#NM Device Not Available",
    "#DF Double Fault",
    "Coprocessor Segment Overrun",
    "#TS Invalid TSS",
    "#NP Segment Not Present",
    "#SS Stack Fault",
    "#GP General Protection Fault",
    "#PF Page Fault",
    "Reserved (15)",
    "#MF x87 FPU Error",
    "#AC Alignment Check",
    "#MC Machine Check",
    "#XM SIMD Exception",
    "#VE Virtualization",
    "Reserved (21)",
    "Reserved (22)",
    "Reserved (23)",
    "Reserved (24)",
    "Reserved (25)",
    "Reserved (26)",
    "Reserved (27)",
    "Reserved (28)",
    "Reserved (29)",
    "#SX Security",
    "Reserved (31)",
];

/// The Rust dispatch function called from `isr_common`.
///
/// `frame` points to the saved register state on the stack.
/// `vector` is the interrupt vector number.
#[unsafe(no_mangle)]
#[allow(dead_code)] // called from assembly
pub extern "C" fn dispatch(frame: *const InterruptFrame, vector: u64) {
    let v = vector as usize;

    // If this is a PIC IRQ (32–47), send EOI.
    if (32..48).contains(&v) {
        let irq = (v - 32) as u8;
        unsafe { super::pic::end_of_interrupt(irq) };
        crate::irq_dispatch(irq);
    } else if v == 128 {
        // System call
        crate::syscall::handle_syscall(frame);
    } else if v < 32 {
        // CPU exception
        let name = EXCEPTION_NAMES[v];
        crate::println!("\n[EXCEPTION] {} (vector {})", name, v);
        if let Some(f) = unsafe { frame.as_ref() } {
            f.dump();
        }
        // For #DF (8) and #MC (18) we can't recover.
        if v == 8 || v == 18 {
            crate::println!("Fatal exception, halting.");
            unsafe {
                loop {
                    core::arch::asm!("cli; hlt", options(nostack, preserves_flags));
                }
            }
        }
    } else {
        crate::println!("[INT] Unexpected interrupt vector {}", v);
    }
}

// ─── Public init ─────────────────────────────────────────────────────────────

/// Map of ISR symbol names → vector numbers, for IDT setup.
/// We store the addresses in a static array via linker symbols.
macro_rules! isr_addr {
    ($v:expr) => {{
        extern "C" {
            #[unsafe(no_mangle)]
            static ISR_START: u8;
        }
        let _ = $v;
        0u64 // placeholder — replaced by the real approach below
    }};
}

/// Return the address of the ISR stub for vector `n`.
///
/// # Safety
/// Must only be called for vectors that have a corresponding `isr_N` symbol.
pub unsafe fn isr_address(n: usize) -> u64 {
    unsafe {
        match n {
            0 => isr_0 as usize as u64,
            1 => isr_1 as usize as u64,
            2 => isr_2 as usize as u64,
            3 => isr_3 as usize as u64,
            4 => isr_4 as usize as u64,
            5 => isr_5 as usize as u64,
            6 => isr_6 as usize as u64,
            7 => isr_7 as usize as u64,
            8 => isr_8 as usize as u64,
            9 => isr_9 as usize as u64,
            10 => isr_10 as usize as u64,
            11 => isr_11 as usize as u64,
            12 => isr_12 as usize as u64,
            13 => isr_13 as usize as u64,
            14 => isr_14 as usize as u64,
            15 => isr_15 as usize as u64,
            16 => isr_16 as usize as u64,
            17 => isr_17 as usize as u64,
            18 => isr_18 as usize as u64,
            19 => isr_19 as usize as u64,
            20 => isr_20 as usize as u64,
            21 => isr_21 as usize as u64,
            22 => isr_22 as usize as u64,
            23 => isr_23 as usize as u64,
            24 => isr_24 as usize as u64,
            25 => isr_25 as usize as u64,
            26 => isr_26 as usize as u64,
            27 => isr_27 as usize as u64,
            28 => isr_28 as usize as u64,
            29 => isr_29 as usize as u64,
            30 => isr_30 as usize as u64,
            31 => isr_31 as usize as u64,
            32 => isr_32 as usize as u64,
            33 => isr_33 as usize as u64,
            34 => isr_34 as usize as u64,
            35 => isr_35 as usize as u64,
            36 => isr_36 as usize as u64,
            37 => isr_37 as usize as u64,
            38 => isr_38 as usize as u64,
            39 => isr_39 as usize as u64,
            40 => isr_40 as usize as u64,
            41 => isr_41 as usize as u64,
            42 => isr_42 as usize as u64,
            43 => isr_43 as usize as u64,
            44 => isr_44 as usize as u64,
            45 => isr_45 as usize as u64,
            46 => isr_46 as usize as u64,
            47 => isr_47 as usize as u64,
            128 => isr_128 as usize as u64,
            _ => 0,
        }
    }
}

// Extern declarations for the assembly stubs.
unsafe extern "C" {
    fn isr_0();  fn isr_1();  fn isr_2();  fn isr_3();  fn isr_4();
    fn isr_5();  fn isr_6();  fn isr_7();  fn isr_8();  fn isr_9();
    fn isr_10(); fn isr_11(); fn isr_12(); fn isr_13(); fn isr_14();
    fn isr_15(); fn isr_16(); fn isr_17(); fn isr_18(); fn isr_19();
    fn isr_20(); fn isr_21(); fn isr_22(); fn isr_23(); fn isr_24();
    fn isr_25(); fn isr_26(); fn isr_27(); fn isr_28(); fn isr_29();
    fn isr_30(); fn isr_31();
    fn isr_32(); fn isr_33(); fn isr_34(); fn isr_35(); fn isr_36();
    fn isr_37(); fn isr_38(); fn isr_39(); fn isr_40(); fn isr_41();
    fn isr_42(); fn isr_43(); fn isr_44(); fn isr_45(); fn isr_46();
    fn isr_47();
    fn isr_128();
}
