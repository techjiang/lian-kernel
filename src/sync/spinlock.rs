//! A simple spinlock: busy-wait on an atomic flag.
//!
//! When interrupts are enabled we also disable them while holding the lock
//! to prevent deadlock from interrupt handlers on the same CPU.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

/// A mutual-exclusion spinlock that also masks interrupts.
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: access is serialized by the spinlock; T: Send is required.
unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    /// Acquire the lock, returning a guard that releases on drop.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        // Save interrupt state and disable.
        let was_enabled = unsafe { irq_save() };
        // Spin until we get the lock.
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin with `pause` to reduce power and avoid pipeline stalls.
            while self.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
        SpinLockGuard {
            lock: self,
            was_enabled,
        }
    }

    /// Try to acquire without spinning.  Returns `None` if already locked.
    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        let was_enabled = unsafe { irq_save() };
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinLockGuard {
                lock: self,
                was_enabled,
            })
        } else {
            unsafe { irq_restore(was_enabled) };
            None
        }
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    was_enabled: bool,
}

impl<T> core::ops::Deref for SpinLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: we hold the lock.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: we hold the lock.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        unsafe { irq_restore(self.was_enabled) };
    }
}

// ─── x86-64 interrupt helpers ───────────────────────────────────────────────

/// Disable interrupts and return whether they were previously enabled.
///
/// # Safety
/// Must only be called from kernel context (ring 0) on x86-64.
#[inline]
pub unsafe fn irq_save() -> bool {
    let flags: u64;
    core::arch::asm!(
        "pushfq",
        "pop {0}",
        "cli",
        out(reg) flags,
        options(nostack, preserves_flags),
    );
    (flags & 0x200) != 0 // IF flag (bit 9)
}

/// Restore interrupt state.  `was_enabled` should come from `irq_save`.
///
/// # Safety
/// Must only be called from kernel context (ring 0) on x86-64.
#[inline]
pub unsafe fn irq_restore(was_enabled: bool) {
    if was_enabled {
        core::arch::asm!("sti", options(nostack, preserves_flags));
    }
    // If was_enabled is false, interrupts stay disabled (we already did `cli`).
}
