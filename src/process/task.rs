//! Task (kernel thread) abstraction.

use crate::arch::x86_64::context::{CpuContext, init_stack};
use crate::memory::PAGE_SIZE;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Task states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

/// Task priorities (lower number = higher priority).
pub const PRIO_DEFAULT: u8 = 128;

/// A kernel task (thread).
pub struct Task {
    /// Saved CPU context (RSP, CR3, etc.).
    pub context: CpuContext,
    /// Task ID (PID).
    pub id: u64,
    /// Task name (for debugging).
    pub name: [u8; 16],
    /// Current state.
    pub state: TaskState,
    /// Priority.
    pub priority: u8,
    /// Total ticks this task has run.
    pub ticks: u64,
    /// Stack memory (owned, boxed slice so it doesn't move).
    pub stack: Option<alloc::boxed::Box<[u8]>>,
}

static NEXT_ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);

impl Task {
    /// Create a new kernel task.
    pub fn new_kernel(name: &str, entry: extern "C" fn() -> !, stack_size: usize) -> Self {
        let id = NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let stack_size = (stack_size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        // Allocate a stack via the frame allocator.
        // We can't use alloc::vec::Vec yet (no heap), so we use a raw
        // byte array allocated on the heap once the heap is ready.
        // For the idle task, we use a static stack.
        let mut stack_buf: alloc::vec::Vec<u8> = alloc::vec![0u8; stack_size];
        let stack_top = stack_buf.as_mut_ptr().wrapping_add(stack_buf.len());

        // Set up the initial stack frame.
        let rsp = unsafe { init_stack(stack_top, entry) };

        // Copy name.
        let mut name_buf = [0u8; 16];
        let name_bytes = name.as_bytes();
        let copy_len = name_bytes.len().min(15);
        name_buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

        let stack_box = stack_buf.into_boxed_slice();

        Task {
            context: CpuContext::new(rsp, 0), // CR3=0 means kernel address space
            id,
            name: name_buf,
            state: TaskState::Ready,
            priority: PRIO_DEFAULT,
            ticks: 0,
            stack: Some(stack_box),
        }
    }

    /// Get the task name as a string slice (up to the first NUL).
    pub fn name_str(&self) -> &str {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(16);
        core::str::from_utf8(&self.name[..end]).unwrap_or("?")
    }

    /// Print task info.
    pub fn dump(&self) {
        crate::println!(
            "  [{:>3}] {:<16} {:?} pri={} ticks={}",
            self.id,
            self.name_str(),
            self.state,
            self.priority,
            self.ticks
        );
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        // The stack Box will be freed automatically.
    }
}
