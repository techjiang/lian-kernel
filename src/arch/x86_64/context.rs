//! CPU context for task switching.
//!
//! We use a stack-based context switch: the callee-saved registers are pushed
//! onto the task's own stack, and only the stack pointer is stored in the
//! `CpuContext`.  The actual switch is done by a `global_asm!` function.

use core::arch::global_asm;

/// Per-task context.  Only the stack pointer is needed — everything else
/// is saved on the task's own stack.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpuContext {
    pub rsp: u64,
    pub cr3: u64,
}

impl CpuContext {
    /// Create an initial context for a new kernel task.
    ///
    /// The stack must already be set up with the entry function address
    /// and any arguments, matching the `ret` sequence in `switch`.
    pub fn new(rsp: u64, cr3: u64) -> Self {
        CpuContext { rsp, cr3 }
    }
}

global_asm!(
    r#"
.section .text
.code64

# void context_switch(u64 *old_rsp, u64 new_rsp)
# rdi = pointer to old_rsp (where to store current RSP)
# rsi = new RSP value
.globl context_switch
.type context_switch, @function
context_switch:
    # Save callee-saved registers.
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15

    # Save current RSP into *old_rsp
    mov [rdi], rsp

    # Load new RSP
    mov rsp, rsi

    # Restore callee-saved registers from the new stack.
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp

    ret
"#,
);

unsafe extern "C" {
    fn context_switch(old_rsp: *mut u64, new_rsp: u64);
}

/// Switch from the current task to the next one.
///
/// `old_ctx` receives the current RSP so we can resume later.
/// `new_ctx` provides the RSP to switch to.
///
/// # Safety
/// The contexts must be valid and not shared with other CPUs.
#[inline(never)]
pub unsafe fn switch(old_ctx: &mut CpuContext, new_ctx: &CpuContext) {
    // Optionally switch CR3 (address space).
    if new_ctx.cr3 != 0 {
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) new_ctx.cr3,
            options(nostack, preserves_flags),
        );
    }
    context_switch(&mut old_ctx.rsp as *mut u64, new_ctx.rsp);
}

/// Set up a new task's stack so that the first `switch` into it will call
/// `entry()`.
///
/// Returns the initial RSP value to store in `CpuContext`.
///
/// # Safety
/// `stack` must be a valid writable region; `stack_top` must be aligned to
/// 16 bytes.
pub unsafe fn init_stack(stack_top: *mut u8, entry: extern "C" fn() -> !) -> u64 {
    // We need to set up the stack as if `context_switch` had just pushed
    // the callee-saved registers and is about to `ret` into `entry`.
    //
    // Stack layout (growing downward):
    //   [ret addr = entry]  <- this is what `ret` will pop
    //   [rbp = 0]
    //   [rbx = 0]
    //   [r12 = 0]
    //   [r13 = 0]
    //   [r14 = 0]
    //   [r15 = 0]
    //
    // The RSP we return points to the "r15" slot (bottom of the frame).

    let mut sp = stack_top as u64;

    // Align to 16 bytes (x86_64 ABI requirement).
    sp &= !0xF;

    // Push the return address (entry point).
    sp -= 8;
    *(sp as *mut u64) = entry as usize as u64;

    // Push callee-saved registers (all zero).
    sp -= 8; *(sp as *mut u64) = 0; // rbp
    sp -= 8; *(sp as *mut u64) = 0; // rbx
    sp -= 8; *(sp as *mut u64) = 0; // r12
    sp -= 8; *(sp as *mut u64) = 0; // r13
    sp -= 8; *(sp as *mut u64) = 0; // r14
    sp -= 8; *(sp as *mut u64) = 0; // r15

    sp
}
