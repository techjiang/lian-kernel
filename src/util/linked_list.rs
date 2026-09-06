//! Intrusive doubly-linked list with no heap allocation.
//!
//! Each node owns its own `Link` field; the list itself just holds head/tail
//! pointers.  This is the standard building block for kernel queues
//! (scheduler run-queue, free-lists, wait-queues, etc.).

use core::ptr;

/// A link in an intrusive list.  Store this inside the struct you want to
/// chain together.
#[derive(Debug)]
pub struct Link {
    pub next: *mut Link,
    pub prev: *mut Link,
}

impl Link {
    /// An unlinked (self-referential-free) node.  Conventionally `next`
    /// and `prev` are null until the node is inserted.
    pub const fn new() -> Self {
        Link {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
        }
    }
}

/// An intrusive doubly-linked list.
pub struct LinkedList {
    head: *mut Link,
    tail: *mut Link,
    len: usize,
}

impl LinkedList {
    pub const fn new() -> Self {
        LinkedList {
            head: ptr::null_mut(),
            tail: ptr::null_mut(),
            len: 0,
        }
    }

    /// Push `node` at the tail.
    ///
    /// # Safety
    /// The caller must guarantee `node` outlives its membership in the list
    /// and that it is not simultaneously in another list.
    pub unsafe fn push_back(&mut self, node: *mut Link) {
        debug_assert!(!node.is_null());
        (*node).next = ptr::null_mut();
        (*node).prev = self.tail;
        if self.tail.is_null() {
            self.head = node;
        } else {
            (*self.tail).next = node;
        }
        self.tail = node;
        self.len += 1;
    }

    /// Push `node` at the head.
    pub unsafe fn push_front(&mut self, node: *mut Link) {
        (*node).prev = ptr::null_mut();
        (*node).next = self.head;
        if self.head.is_null() {
            self.tail = node;
        } else {
            (*self.head).prev = node;
        }
        self.head = node;
        self.len += 1;
    }

    /// Remove and return the front node (or null if empty).
    pub unsafe fn pop_front(&mut self) -> *mut Link {
        if self.head.is_null() {
            return ptr::null_mut();
        }
        let node = self.head;
        self.head = (*node).next;
        if self.head.is_null() {
            self.tail = ptr::null_mut();
        } else {
            (*self.head).prev = ptr::null_mut();
        }
        self.len -= 1;
        node
    }

    /// Remove `node` from wherever it is in the list.
    pub unsafe fn remove(&mut self, node: *mut Link) {
        let prev = (*node).prev;
        let next = (*node).next;
        if prev.is_null() {
            self.head = next;
        } else {
            (*prev).next = next;
        }
        if next.is_null() {
            self.tail = prev;
        } else {
            (*next).prev = prev;
        }
        (*node).prev = ptr::null_mut();
        (*node).next = ptr::null_mut();
        self.len -= 1;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn head(&self) -> *mut Link {
        self.head
    }

    /// Iterate over node pointers (front → back).
    pub fn iter(&self) -> LinkedListIter {
        LinkedListIter {
            current: self.head,
        }
    }
}

pub struct LinkedListIter {
    current: *mut Link,
}

impl Iterator for LinkedListIter {
    type Item = *mut Link;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }
        let ret = self.current;
        // SAFETY: node was inserted by `push_*`, so `next` is a valid link.
        self.current = unsafe { (*ret).next };
        Some(ret)
    }
}
