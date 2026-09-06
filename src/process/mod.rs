//! Process and scheduling subsystem.
//!
//! Provides:
//!   • `task`      — Task struct (a kernel thread)
//!   • `scheduler` — Round-robin preemptive scheduler

pub mod task;
pub mod scheduler;
