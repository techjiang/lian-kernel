//! Physical frame allocator.
//!
//! Uses a bitmap to track free/used 4 KiB frames.  The memory map from
//! the multiboot2 info is parsed at init time to determine usable RAM.

use crate::util::bitmap::Bitmap;
use crate::memory::{PAGE_SIZE, PhysAddr};
use crate::sync::spinlock::SpinLock;

/// Maximum frames representable (2 Mib of bitmap = 128 GiB of RAM).
/// We use a static array of u64s; 32768 words × 64 bits × 4 KiB = 8 GiB.
/// Extend as needed.
const BITMAP_WORDS: usize = 32768;

static BITMAP_STORAGE: SpinLock<BitmapStorage> = SpinLock::new(BitmapStorage::new());

struct BitmapStorage {
    words: [u64; BITMAP_WORDS],
    bitmap: Option<Bitmap>,
    base: PhysAddr,   // physical address of frame 0
    total: usize,     // total frames
}

impl BitmapStorage {
    const fn new() -> Self {
        BitmapStorage {
            words: [0; BITMAP_WORDS],
            bitmap: None,
            base: 0,
            total: 0,
        }
    }
}

/// Region of usable physical memory.
#[derive(Clone, Copy, Debug)]
pub struct MemoryRegion {
    pub start: PhysAddr,
    pub len: usize,
    pub usable: bool,
}

/// Initialize the frame allocator.
///
/// # Safety
/// Must be called once during boot.  `regions` should come from the
/// multiboot2 memory map.
pub unsafe fn init(regions: &[MemoryRegion]) {
    let mut storage = BITMAP_STORAGE.lock();

    // Find the highest usable address to size the bitmap.
    let mut max_addr: PhysAddr = 0;
    for region in regions {
        if region.usable {
            let end = region.start + region.len;
            if end > max_addr {
                max_addr = end;
            }
        }
    }

    let total_frames = max_addr / PAGE_SIZE;
    let words_needed = (total_frames + 63) / 64;

    if words_needed > BITMAP_WORDS {
        // Clamp to what we can represent.
        crate::println!(
            "[mem] Warning: {} frames > bitmap capacity, clamping to {}",
            total_frames,
            BITMAP_WORDS * 64
        );
    }

    let words = if words_needed > BITMAP_WORDS { BITMAP_WORDS } else { words_needed };
    let actual_total = words * 64;

    // Create the bitmap.
    let bitmap_ref: &'static mut [u64] = {
        // SAFETY: BITMAP_STORAGE is a static; the reference is valid for
        // 'static because the storage never moves and we never deallocate.
        core::slice::from_raw_parts_mut(storage.words.as_mut_ptr(), words)
    };

    let mut bitmap = Bitmap::new(bitmap_ref, actual_total);

    // Mark ALL frames as used initially.
    for i in 0..actual_total {
        bitmap.set_used(i);
    }

    // Mark usable regions as free.
    for region in regions {
        if !region.usable {
            continue;
        }
        let start_frame = region.start / PAGE_SIZE;
        let end_frame = (region.start + region.len) / PAGE_SIZE;
        for i in start_frame..end_frame.min(actual_total) {
            bitmap.set_free(i);
        }
    }

    // Reserve the first 1 MiB (BIOS, kernel image, etc.).
    let one_mb_frames = (1024 * 1024) / PAGE_SIZE;
    for i in 0..one_mb_frames.min(actual_total) {
        bitmap.set_used(i);
    }

    storage.base = 0;
    storage.total = actual_total;

    let free = bitmap.count_free();
    storage.bitmap = Some(bitmap);

    crate::println!(
        "[mem] Frame allocator initialized: {} frames total, {} free ({} MiB)",
        actual_total,
        free,
        free * PAGE_SIZE / (1024 * 1024)
    );
}

/// Allocate a single physical frame.  Returns `None` if OOM.
pub fn alloc_frame() -> Option<PhysAddr> {
    let mut storage = BITMAP_STORAGE.lock();
    if let Some(ref mut bitmap) = storage.bitmap {
        if let Some(idx) = bitmap.find_free() {
            bitmap.set_used(idx);
            return Some(storage.base + idx * PAGE_SIZE);
        }
    }
    None
}

/// Allocate `n` contiguous frames.  Returns the physical address of the
/// first frame, or `None`.
pub fn alloc_contiguous(n: usize) -> Option<PhysAddr> {
    if n == 0 {
        return Some(0);
    }
    if n == 1 {
        return alloc_frame();
    }

    let mut storage = BITMAP_STORAGE.lock();
    if let Some(ref mut bitmap) = storage.bitmap {
        // Linear scan for `n` consecutive free frames.
        let mut start = None;
        let mut count = 0;
        for i in 0..bitmap.total() {
            if bitmap.is_free(i) {
                if start.is_none() {
                    start = Some(i);
                }
                count += 1;
                if count == n {
                    let s = start.unwrap();
                    for j in s..s + n {
                        bitmap.set_used(j);
                    }
                    return Some(storage.base + s * PAGE_SIZE);
                }
            } else {
                start = None;
                count = 0;
            }
        }
    }
    None
}

/// Free a previously allocated frame.
pub fn free_frame(addr: PhysAddr) {
    let mut storage = BITMAP_STORAGE.lock();
    let base = storage.base;
    if let Some(ref mut bitmap) = storage.bitmap {
        let idx = (addr - base) / PAGE_SIZE;
        bitmap.set_free(idx);
    }
}

/// Number of free frames.
pub fn free_frames() -> usize {
    let storage = BITMAP_STORAGE.lock();
    if let Some(ref bitmap) = storage.bitmap {
        bitmap.count_free()
    } else {
        0
    }
}
