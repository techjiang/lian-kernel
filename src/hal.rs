//! Hardware Abstraction Layer (HAL).
//!
//! Defines traits that each architecture must implement.  The current
//! x86_64 implementation lives in `arch/x86_64`.  Future ports (ARM64,
//! RISC-V) will implement the same traits.

/// Architecture operations that every supported ISA must provide.
pub trait ArchOps: Sync {
    /// Disable interrupts and return the previous state.
    unsafe fn irq_save() -> bool;
    /// Restore interrupt state.
    unsafe fn irq_restore(was_enabled: bool);
    /// Enable interrupts.
    unsafe fn enable_interrupts();
    /// Disable interrupts.
    unsafe fn disable_interrupts();
    /// Halt the CPU.
    unsafe fn halt();
    /// Read the page-table root (CR3 on x86_64).
    fn read_cr3() -> u64;
    /// Write the page-table root.
    unsafe fn write_cr3(val: u64);
    /// Invalidate a TLB entry.
    unsafe fn invalidate_tlb(vaddr: usize);
}

/// Memory management operations.
pub trait MemoryOps: Sync {
    /// Page size in bytes.
    const PAGE_SIZE: usize;
    /// Initialize paging.
    unsafe fn init_paging();
    /// Map a virtual page to a physical frame.
    unsafe fn map_page(vaddr: usize, paddr: usize, flags: u64) -> Result<(), &'static str>;
    /// Translate a virtual address to physical.
    fn translate(vaddr: usize) -> Option<usize>;
    /// Allocate a physical frame.
    fn alloc_frame() -> Option<usize>;
    /// Free a physical frame.
    fn free_frame(paddr: usize);
}

/// Interrupt controller operations.
pub trait InterruptOps: Sync {
    /// Initialize the interrupt controller.
    unsafe fn init_pic();
    /// Send end-of-interrupt.
    unsafe fn end_of_interrupt(irq: u8);
    /// Mask (disable) an IRQ.
    unsafe fn mask_irq(irq: u8);
    /// Unmask (enable) an IRQ.
    unsafe fn unmask_irq(irq: u8);
}

// ─── x86_64 implementation ──────────────────────────────────────────────────

use crate::arch::x86_64;

impl ArchOps for crate::arch::X86_64 {
    unsafe fn irq_save() -> bool {
        crate::sync::spinlock::irq_save()
    }

    unsafe fn irq_restore(was_enabled: bool) {
        crate::sync::spinlock::irq_restore(was_enabled)
    }

    unsafe fn enable_interrupts() {
        x86_64::enable_interrupts();
    }

    unsafe fn disable_interrupts() {
        x86_64::disable_interrupts();
    }

    unsafe fn halt() {
        x86_64::halt();
    }

    fn read_cr3() -> u64 {
        unsafe { x86_64::read_cr3() }
    }

    unsafe fn write_cr3(val: u64) {
        x86_64::write_cr3(val);
    }

    unsafe fn invalidate_tlb(vaddr: usize) {
        crate::memory::page::invlpg(vaddr);
    }
}
