//! Kernel heap allocator.
//!
//! Uses a simple fixed-size block allocator (free-list) for the kernel heap.
//! Memory is backed by frame allocation and mapped into the heap region.

use crate::memory::{PAGE_SIZE, VirtAddr};
use crate::sync::spinlock::SpinLock;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

/// Heap starts at this virtual address and grows upward.
const HEAP_START: VirtAddr = 0xFFFF_8000_0000_0000;

/// Initial heap size: 64 pages = 256 KiB.
const INITIAL_PAGES: usize = 64;

/// Minimum block size: 16 bytes.
const MIN_BLOCK: usize = 16;

/// Block header for the free list.
#[repr(C)]
struct BlockHeader {
    size: usize,
    next: *mut BlockHeader,
}

impl BlockHeader {
    const SIZE: usize = core::mem::size_of::<BlockHeader>();
}

/// The kernel heap state.
struct Heap {
    /// End of currently mapped heap.
    heap_end: VirtAddr,
    /// Head of the free list.
    free_head: *mut BlockHeader,
}

// SAFETY: The heap is only accessed while holding the SpinLock.
unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}

impl Heap {
    const fn new() -> Self {
        Heap {
            heap_end: HEAP_START,
            free_head: ptr::null_mut(),
        }
    }
}

static HEAP: SpinLock<Heap> = SpinLock::new(Heap::new());

/// Allocate `size` bytes from the heap by extending the mapped region.
unsafe fn extend_heap(size: usize) -> Result<*mut u8, &'static str> {
    let mut heap = HEAP.lock();
    let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;

    for _ in 0..pages {
        let frame = crate::memory::frame::alloc_frame()
            .ok_or("OOM in frame allocator")?;

        // Map the frame into the heap region.
        crate::memory::page::map_page(
            heap.heap_end,
            frame,
            crate::memory::page::PRESENT | crate::memory::page::WRITABLE,
        )?;

        heap.heap_end += PAGE_SIZE;
    }

    let alloc_end = heap.heap_end;
    // We allocated `pages * PAGE_SIZE` bytes at the old `heap_end` position.
    // But we need to track the start of this allocation.
    let start = alloc_end - pages * PAGE_SIZE;
    Ok(start as *mut u8)
}

/// Free-list allocator implementation.
unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        // This is called through the `LockedHeap` wrapper below.
        // Not directly used.
        ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Not directly used.
    }
}

/// Locked heap wrapper implementing `GlobalAlloc`.
pub struct LockedHeap;

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // Round up size to at least MIN_BLOCK and to alignment.
        let alloc_size = {
            let s = size.max(MIN_BLOCK);
            let rem = s % align;
            if rem == 0 { s } else { s + align - rem }
        };

        let mut heap = HEAP.lock();

        // First-fit search the free list.
        let mut prev: *mut BlockHeader = ptr::null_mut();
        let mut cur = heap.free_head;
        while !cur.is_null() {
            if (*cur).size >= alloc_size + BlockHeader::SIZE {
                // Split this block.
                let remaining = (*cur).size - alloc_size - BlockHeader::SIZE;
                if remaining >= MIN_BLOCK {
                    // Create a new block from the remainder.
                    let new_block = (cur as *mut u8).add(alloc_size + BlockHeader::SIZE) as *mut BlockHeader;
                    (*new_block).size = remaining;
                    (*new_block).next = (*cur).next;

                    (*cur).size = alloc_size;
                    (*cur).next = ptr::null_mut(); // mark as allocated

                    // Update the free list.
                    if prev.is_null() {
                        heap.free_head = new_block;
                    } else {
                        (*prev).next = new_block;
                    }
                } else {
                    // Use the whole block.
                    if prev.is_null() {
                        heap.free_head = (*cur).next;
                    } else {
                        (*prev).next = (*cur).next;
                    }
                    (*cur).next = ptr::null_mut();
                }

                // Return the data area (after the header).
                return (cur as *mut u8).add(BlockHeader::SIZE);
            }
            prev = cur;
            cur = (*cur).next;
        }

        // No free block found; extend the heap.
        drop(heap);
        match extend_heap(alloc_size + BlockHeader::SIZE) {
            Ok(ptr) => {
                let header = ptr as *mut BlockHeader;
                (*header).size = alloc_size;
                (*header).next = ptr::null_mut();
                // Return data area.
                ptr.add(BlockHeader::SIZE)
            }
            Err(msg) => {
                crate::println!("[heap] alloc failed: {} (requested {} bytes)", msg, size);
                ptr::null_mut()
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let block = ptr.sub(BlockHeader::SIZE) as *mut BlockHeader;
        let mut heap = HEAP.lock();
        (*block).next = heap.free_head;
        heap.free_head = block;
    }
}

/// Initialize the kernel heap.
///
/// # Safety
/// Must be called once during boot, after paging and the frame allocator
/// are initialized.
pub unsafe fn init() {
    // Extend the heap by the initial number of pages.
    let total = INITIAL_PAGES * PAGE_SIZE;
    match extend_heap(total) {
        Ok(ptr) => {
            let mut heap = HEAP.lock();
            let header = ptr as *mut BlockHeader;
            (*header).size = total - BlockHeader::SIZE;
            (*header).next = ptr::null_mut();
            heap.free_head = header;

            crate::println!(
                "[mem] Kernel heap initialized: {} KiB at {:#x}",
                total / 1024,
                ptr as usize
            );
        }
        Err(msg) => {
            crate::println!("[mem] Heap init failed: {}", msg);
        }
    }
}

// ─── Global allocator registration ────────────────────────────────────────────
// The global allocator is declared in lib.rs:
//   #[global_allocator]
//   static ALLOCATOR: memory::heap::LockedHeap = memory::heap::LockedHeap;
