//! Memory management subsystem.
//!
//! Provides:
//!   • `frame` — physical frame (page) allocator
//!   • `page`  — virtual memory / 4-level paging
//!   • `heap`  — kernel heap allocator

pub mod frame;
pub mod page;
pub mod heap;

/// Page size: 4 KiB.
pub const PAGE_SIZE: usize = 4096;

/// Physical address.
pub type PhysAddr = usize;

/// Virtual address.
pub type VirtAddr = usize;
