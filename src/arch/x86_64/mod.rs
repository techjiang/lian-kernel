//! x86_64 architecture module.

pub mod port;
pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod pic;
pub mod timer;
pub mod keyboard;
pub mod context;

/// Read the CR3 register (page-table root).
#[inline]
pub unsafe fn read_cr3() -> u64 {
    let val: u64;
    core::arch::asm!("mov {0}, cr3", out(reg) val, options(nostack, preserves_flags));
    val
}

/// Write the CR3 register.
#[inline]
pub unsafe fn write_cr3(val: u64) {
    core::arch::asm!("mov cr3, {0}", in(reg) val, options(nostack, preserves_flags));
}

/// Read the CR2 register (page-fault address).
#[inline]
pub unsafe fn read_cr2() -> u64 {
    let val: u64;
    core::arch::asm!("mov {0}, cr2", out(reg) val, options(nostack, preserves_flags));
    val
}

/// Enable interrupts.
#[inline]
pub unsafe fn enable_interrupts() {
    core::arch::asm!("sti", options(nostack, preserves_flags));
}

/// Disable interrupts.
#[inline]
pub unsafe fn disable_interrupts() {
    core::arch::asm!("cli", options(nostack, preserves_flags));
}

/// Halt the CPU until the next interrupt.
#[inline]
pub unsafe fn halt() {
    core::arch::asm!("hlt", options(nostack, preserves_flags));
}

/// Read the RSP register.
#[inline]
pub fn read_rsp() -> u64 {
    let rsp: u64;
    unsafe { core::arch::asm!("mov {0}, rsp", out(reg) rsp, options(nostack, preserves_flags)) };
    rsp
}

/// Get the current CPU's RSP (useful for setting up the TSS RSP0).
#[inline]
pub fn current_stack_pointer() -> usize {
    read_rsp() as usize
}
