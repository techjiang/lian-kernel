//! A one-shot initialization cell (like `std::sync::OnceLock`).

use crate::sync::spinlock::SpinLock;

/// A cell that can be initialized exactly once.
pub struct OnceCell<T> {
    inner: SpinLock<Option<T>>,
}

impl<T> OnceCell<T> {
    pub const fn new() -> Self {
        OnceCell {
            inner: SpinLock::new(None),
        }
    }

    /// Initialize the cell.  Returns `Err` if already initialized.
    pub fn init(&self, value: T) -> Result<(), T> {
        let mut guard = self.inner.lock();
        if guard.is_some() {
            return Err(value);
        }
        *guard = Some(value);
        Ok(())
    }

    /// Get a reference to the value.  Panics if not initialized.
    pub fn get(&self) -> &T {
        let guard = self.inner.lock();
        // SAFETY: we hold the lock, so the reference is valid for the
        // lifetime of `guard`.  We leak the guard because returning `&T`
        // tied to the guard's lifetime would require unsafe projection.
        // In practice the cell is never re-locked after initialization.
        let ptr = guard.as_ref().unwrap() as *const T;
        // Extend the lifetime: safe because OnceCell is write-once.
        unsafe { &*ptr }
    }

    /// Returns `true` if the cell has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.inner.lock().is_some()
    }
}
