//! A simple bitmap allocator, used by the physical frame allocator.

pub struct Bitmap {
    /// Each bit represents one frame (1 = free, 0 = used).
    bits: &'static mut [u64],
    /// Total number of frames represented by the bitmap.
    total: usize,
}

impl Bitmap {
    /// Wrap a pre-allocated `bits` buffer.
    ///
    /// # Safety
    /// `bits` must point to a valid writable region for the lifetime of the
    /// kernel.
    pub unsafe fn new(bits: &'static mut [u64], total: usize) -> Self {
        // Mark every bit as "free" initially.
        for word in bits.iter_mut() {
            *word = u64::MAX;
        }
        // Mask out bits beyond `total` so they are never returned as free.
        let last_word = total / 64;
        let remainder = total % 64;
        if remainder != 0 && last_word < bits.len() {
            let mask = (1u64 << remainder) - 1;
            bits[last_word] &= mask;
        }
        Bitmap { bits, total }
    }

    #[inline]
    pub fn total(&self) -> usize {
        self.total
    }

    /// Mark frame `idx` as used.
    #[inline]
    pub fn set_used(&mut self, idx: usize) {
        if idx >= self.total {
            return;
        }
        let word = idx / 64;
        let bit = idx % 64;
        self.bits[word] &= !(1u64 << bit);
    }

    /// Mark frame `idx` as free.
    #[inline]
    pub fn set_free(&mut self, idx: usize) {
        if idx >= self.total {
            return;
        }
        let word = idx / 64;
        let bit = idx % 64;
        self.bits[word] |= 1u64 << bit;
    }

    #[inline]
    pub fn is_free(&self, idx: usize) -> bool {
        if idx >= self.total {
            return false;
        }
        let word = idx / 64;
        let bit = idx % 64;
        (self.bits[word] >> bit) & 1 == 1
    }

    /// Find the first free frame and return its index, or `None`.
    pub fn find_free(&self) -> Option<usize> {
        for (wi, word) in self.bits.iter().enumerate() {
            if *word != 0 {
                // trailing_ones + 1 gives the position of the first set bit
                let bit = word.trailing_zeros() as usize;
                let idx = wi * 64 + bit;
                if idx < self.total {
                    return Some(idx);
                }
            }
        }
        None
    }

    /// Count total free frames.
    pub fn count_free(&self) -> usize {
        let mut count = 0;
        for word in self.bits.iter() {
            count += word.count_ones() as usize;
        }
        // Subtract the masked-out bits in the last word.
        let last = self.total / 64;
        let rem = self.total % 64;
        if rem != 0 && last < self.bits.len() {
            let mask = (1u64 << rem) - 1;
            count -= (self.bits[last] & !mask).count_ones() as usize;
        }
        count
    }
}
