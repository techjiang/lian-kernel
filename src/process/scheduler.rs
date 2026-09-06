//! Preemptive round-robin scheduler.
//!
//! On each timer tick, the scheduler preempts the current task if its
//! time quantum is exhausted.  Blocked tasks are skipped.

use crate::process::task::{Task, TaskState};
use crate::arch::x86_64::context;
use crate::sync::spinlock::SpinLock;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Time quantum in ticks before preemption.
const TIME_QUANTUM: u64 = 5;

struct Scheduler {
    /// Ready queue.
    ready: VecDeque<Box<Task>>,
    /// Currently running task.
    current: Option<Box<Task>>,
    /// Terminated tasks (to be dropped).
    reaped: Vec<u64>,
}

impl Scheduler {
    const fn new() -> Self {
        Scheduler {
            ready: VecDeque::new(),
            current: None,
            reaped: Vec::new(),
        }
    }
}

static SCHED: SpinLock<Scheduler> = SpinLock::new(Scheduler::new());

/// Initialize the scheduler and create the idle task.
pub fn init() {
    // The "idle" task just halts in a loop.
    extern "C" fn idle() -> ! {
        loop {
            unsafe { core::arch::asm!("hlt", options(nostack, preserves_flags)) };
        }
    }

    let idle_task = Task::new_kernel("idle", idle, 16384);
    crate::println!("[sched] Idle task created (pid {})", idle_task.id);

    // We don't actually switch to idle until the first `schedule()` call.
    // For now, just store it.
    let mut sched = SCHED.lock();
    sched.ready.push_back(Box::new(idle_task));
}

/// Spawn a new kernel task.
pub fn spawn(name: &str, entry: extern "C" fn() -> !, stack_size: usize) -> u64 {
    let task = Task::new_kernel(name, entry, stack_size);
    let id = task.id;
    let mut sched = SCHED.lock();
    sched.ready.push_back(Box::new(task));
    crate::println!("[sched] Spawned task '{}' (pid {})", name, id);
    id
}

/// Timer tick handler — called from the PIT interrupt.
pub fn on_timer_tick() {
    let mut sched = SCHED.lock();

    if let Some(ref mut current) = sched.current {
        current.ticks += 1;
        if current.ticks >= TIME_QUANTUM {
            // Time to preempt.
            if let Some(mut task) = sched.current.take() {
                task.state = TaskState::Ready;
                task.ticks = 0;
                sched.ready.push_back(task);
            }
            // Pick the next task.
            sched.current = sched.ready.pop_front();
            if let Some(ref mut next) = sched.current {
                next.state = TaskState::Running;
            }
        }
    } else {
        // No current task — pick one.
        sched.current = sched.ready.pop_front();
        if let Some(ref mut next) = sched.current {
            next.state = TaskState::Running;
        }
    }
}

/// Print all tasks (for debugging).
pub fn dump_tasks() {
    let sched = SCHED.lock();
    crate::println!("--- Task List ---");
    if let Some(ref cur) = sched.current {
        cur.dump();
    }
    for task in sched.ready.iter() {
        task.dump();
    }
    crate::println!("--- End ---");
}

/// Total number of tasks (current + ready).
pub fn task_count() -> usize {
    let sched = SCHED.lock();
    let mut count = sched.ready.len();
    if sched.current.is_some() {
        count += 1;
    }
    count
}
