//! Virtual memory management — 4-level paging for x86_64.
//!
//! 48-bit virtual addresses, 4 KiB pages, 512 entries per table.

use crate::memory::{PAGE_SIZE, PhysAddr, VirtAddr};
use crate::arch::x86_64;
use crate::sync::spinlock::SpinLock;
use core::ptr;

// ─── Page table entry flags ──────────────────────────────────────────────────

pub const PRESENT: u64 = 1 << 0;
pub const WRITABLE: u64 = 1 << 1;
pub const USER_ACCESS: u64 = 1 << 2;
pub const WRITE_THROUGH: u64 = 1 << 3;
pub const NO_CACHE: u64 = 1 << 4;
pub const HUGE_PAGE: u64 = 1 << 7;
pub const GLOBAL: u64 = 1 << 8;
pub const NO_EXECUTE: u64 = 1 << 63;

const ENTRIES: usize = 512;

/// A generic page table entry (PML4 → PDPT → PD → PT).
#[repr(align(4096))]
pub struct PageTable {
    pub entries: [u64; ENTRIES],
}

impl PageTable {
    /// Create an empty table (all entries zero).
    pub const fn zero() -> Self {
        PageTable {
            entries: [0; ENTRIES],
        }
    }

    /// Get a reference to an entry.
    #[inline]
    pub fn entry(&self, idx: usize) -> u64 {
        self.entries[idx]
    }

    /// Set an entry.
    #[inline]
    pub fn set_entry(&mut self, idx: usize, value: u64) {
        self.entries[idx] = value;
    }

    /// Clear an entry.
    #[inline]
    pub fn clear_entry(&mut self, idx: usize) {
        self.entries[idx] = 0;
    }
}

// ─── Address decomposition ───────────────────────────────────────────────────

/// Decompose a 48-bit canonical virtual address into table indices.
#[derive(Clone, Copy, Debug)]
pub struct PageIndices {
    pub pml4: usize, // level 4
    pub pdpt: usize, // level 3
    pub pd: usize,   // level 2
    pub pt: usize,    // level 1
}

impl PageIndices {
    pub fn from_addr(vaddr: VirtAddr) -> Self {
        PageIndices {
            pml4: (vaddr >> 39) & 0x1FF,
            pdpt: (vaddr >> 30) & 0x1FF,
            pd: (vaddr >> 21) & 0x1FF,
            pt: (vaddr >> 12) & 0x1FF,
        }
    }
}

// ─── Active page tables ───────────────────────────────────────────────────────

/// The active PML4 table.  Allocated as a static so it persists.
static mut KERNEL_PML4: PageTable = PageTable::zero();

/// Recursive index: we map PML4[511] → itself so we can access all tables
/// through a recursive virtual address.
const RECURSIVE_IDX: usize = 511;

/// The recursive virtual address base.
const RECURSIVE_BASE: VirtAddr = 0xFFFF_FF80_0000_0000; // depends on RECURSIVE_IDX

static PML4_LOCK: SpinLock<()> = SpinLock::new(());

/// Initialize kernel paging.
///
/// Maps:
///   • Physical 0 → Virt 0 (identity map first 4 GiB for low memory)
///   • Kernel image at 0x100000 identity-mapped
///   • Higher half (optional, not implemented yet)
///
/// # Safety
/// Must be called once during boot.  After this, the old page tables are
/// replaced and all pointer references must use the new mapping.
pub unsafe fn init() {
    let _lock = PML4_LOCK.lock();

    // Allocate 4 PD tables and a PDPT statically.
    static mut PD0: PageTable = PageTable::zero();
    static mut PD1: PageTable = PageTable::zero();
    static mut PD2: PageTable = PageTable::zero();
    static mut PD3: PageTable = PageTable::zero();
    static mut PDPT: PageTable = PageTable::zero();

    unsafe {
        // Set up recursive mapping: PML4[511] → PML4 itself.
        let pml4_phys = core::ptr::addr_of!(KERNEL_PML4) as PhysAddr;
        KERNEL_PML4.entries[RECURSIVE_IDX] = pml4_phys as u64 | PRESENT | WRITABLE;

        // Use 1 GiB huge pages for identity mapping first 4 GiB.
        let pdpt_phys = core::ptr::addr_of!(PDPT) as PhysAddr;
        KERNEL_PML4.entries[0] = pdpt_phys as u64 | PRESENT | WRITABLE;

        // Map 4 GiB with 1 GiB huge pages.
        for i in 0..4 {
            PDPT.entries[i] = (i as u64 * 1024 * 1024 * 1024) | PRESENT | WRITABLE | HUGE_PAGE;
        }

        // Load the new PML4 into CR3.
        x86_64::write_cr3(pml4_phys as u64);
    }

    crate::println!("[mem] Paging initialized (4 GiB identity-mapped, recursive PML4[{}])", RECURSIVE_IDX);
}

/// Map a virtual page to a physical frame with the given flags.
///
/// # Safety
/// Modifying the active page tables can invalidate existing mappings.
pub unsafe fn map_page(vaddr: VirtAddr, paddr: PhysAddr, flags: u64) -> Result<(), &'static str> {
    let _lock = PML4_LOCK.lock();
    let indices = PageIndices::from_addr(vaddr);

    let pml4 = unsafe { &mut *core::ptr::addr_of_mut!(KERNEL_PML4) };

    // PML4 → PDPT
    let pdpt_entry = pml4.entries[indices.pml4];
    let pdpt: *mut PageTable = if pdpt_entry & PRESENT != 0 {
        (pdpt_entry & 0x000F_FFFF_FFFF_F000) as *mut PageTable
    } else {
        let new_table = alloc_table()?;
        pml4.entries[indices.pml4] = (new_table as u64) | PRESENT | WRITABLE | flags;
        new_table
    };

    // PDPT → PD
    let pdpt = unsafe { &mut *pdpt };
    let pd_entry = pdpt.entries[indices.pdpt];
    let pd: *mut PageTable = if pd_entry & PRESENT != 0 {
        if pd_entry & HUGE_PAGE != 0 {
            return Ok(());
        }
        (pd_entry & 0x000F_FFFF_FFFF_F000) as *mut PageTable
    } else {
        let new_table = alloc_table()?;
        pdpt.entries[indices.pdpt] = (new_table as u64) | PRESENT | WRITABLE | flags;
        new_table
    };

    // PD → PT
    let pd = unsafe { &mut *pd };
    let pt_entry = pd.entries[indices.pd];
    let pt: *mut PageTable = if pt_entry & PRESENT != 0 {
        if pt_entry & HUGE_PAGE != 0 {
            return Ok(());
        }
        (pt_entry & 0x000F_FFFF_FFFF_F000) as *mut PageTable
    } else {
        let new_table = alloc_table()?;
        pd.entries[indices.pd] = (new_table as u64) | PRESENT | WRITABLE | flags;
        new_table
    };

    // PT → physical frame
    let pt = unsafe { &mut *pt };
    pt.entries[indices.pt] = (paddr as u64) | PRESENT | flags;

    // Flush TLB for this page.
    invlpg(vaddr);

    Ok(())
}

/// Allocate a 4 KiB-aligned page table from the frame allocator.
unsafe fn alloc_table() -> Result<*mut PageTable, &'static str> {
    match crate::memory::frame::alloc_frame() {
        Some(paddr) => {
            let table = paddr as *mut PageTable;
            ptr::write_bytes(table as *mut u8, 0, PAGE_SIZE);
            Ok(table)
        }
        None => Err("out of physical memory"),
    }
}

/// Invalidate a TLB entry for the given virtual address.
#[inline]
pub unsafe fn invlpg(vaddr: VirtAddr) {
    core::arch::asm!("invlpg [{}]", in(reg) vaddr, options(nostack, preserves_flags));
}

/// Translate a virtual address to a physical address (walks the page tables).
pub fn translate(vaddr: VirtAddr) -> Option<PhysAddr> {
    let indices = PageIndices::from_addr(vaddr);
    unsafe {
        let pml4 = &*core::ptr::addr_of!(KERNEL_PML4);
        let pdpt_entry = pml4.entries[indices.pml4];
        if pdpt_entry & PRESENT == 0 {
            return None;
        }
        let pdpt = (pdpt_entry & 0x000F_FFFF_FFFF_F000) as *const PageTable;
        let pd_entry = (*pdpt).entries[indices.pdpt];
        if pd_entry & PRESENT == 0 {
            return None;
        }
        if pd_entry & HUGE_PAGE != 0 {
            return Some((pd_entry & 0xFFFFC0000000) as PhysAddr + (vaddr & 0x3FFFFFFF));
        }
        let pd = (pd_entry & 0x000F_FFFF_FFFF_F000) as *const PageTable;
        let pt_entry = (*pd).entries[indices.pd];
        if pt_entry & PRESENT == 0 {
            return None;
        }
        if pt_entry & HUGE_PAGE != 0 {
            return Some((pt_entry & 0x000F_FFFF_FFE00000) as PhysAddr + (vaddr & 0x1FFFFF));
        }
        let pt = (pt_entry & 0x000F_FFFF_FFFF_F000) as *const PageTable;
        let pte = (*pt).entries[indices.pt];
        if pte & PRESENT == 0 {
            return None;
        }
        Some((pte & 0x000F_FFFF_FFFF_F000) as PhysAddr + (vaddr & 0xFFF))
    }
}

/// Get the physical address of the kernel PML4 (for CR3 switching).
pub fn pml4_phys() -> PhysAddr {
    unsafe { core::ptr::addr_of!(KERNEL_PML4) as PhysAddr }
}
