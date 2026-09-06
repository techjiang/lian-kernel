//! A wrapper type for volatile memory-mapped I/O.
//!
//! Rust's optimizer may remove or reorder memory accesses to `mmem`-mapped
//! regions.  Wrapping a raw pointer in `Volatile<T>` ensures every read or
//! write is emitted as a real load/store instruction.

use core::ptr;

/// Volatile wrapper around a single value.
#[repr(transparent)]
pub struct Volatile<T> {
    value: T,
}

impl<T> Volatile<T> {
    /// Create a new `Volatile` wrapping the given value (by copy).
    #[inline]
    pub const fn new(value: T) -> Self {
        Volatile { value }
    }

    /// Read the current value without going through the optimizer.
    #[inline]
    pub fn read(&self) -> T
    where
        T: Copy,
    {
        // SAFETY: `self.value` is a valid, properly aligned `T`.  The
        // `read_volatile` prevents the compiler from caching or eliding
        // the load.
        unsafe { ptr::read_volatile(&self.value as *const T) }
    }

    /// Write a new value without going through the optimizer.
    #[inline]
    pub fn write(&mut self, value: T)
    where
        T: Copy,
    {
        // SAFETY: same reasoning as `read`.
        unsafe {
            ptr::write_volatile(&mut self.value as *mut T, value);
        }
    }

    /// Update the value through a closure, returning the old value.
    #[inline]
    pub fn update<F>(&mut self, f: F) -> T
    where
        T: Copy,
        F: FnOnce(T) -> T,
    {
        let old = self.read();
        self.write(f(old));
        old
    }
}

/// A volatile pointer — useful for MMIO register arrays.
#[derive(Debug, Clone, Copy)]
pub struct VolatilePtr<T> {
    ptr: *mut T,
}

impl<T> VolatilePtr<T> {
    /// Construct from a raw address.
    #[inline]
    pub const fn new(addr: usize) -> Self {
        VolatilePtr {
            ptr: addr as *mut T,
        }
    }

    /// Read one element at `index`.
    #[inline]
    pub unsafe fn read_at(&self, index: usize) -> T
    where
        T: Copy,
    {
        ptr::read_volatile(self.ptr.add(index))
    }

    /// Write one element at `index`.
    #[inline]
    pub unsafe fn write_at(&self, index: usize, value: T)
    where
        T: Copy,
    {
        ptr::write_volatile(self.ptr.add(index), value);
    }

    /// Raw underlying pointer (for offset calculations).
    #[inline]
    pub const fn as_ptr(&self) -> *mut T {
        self.ptr
    }
}
