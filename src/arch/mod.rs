//! Architecture-specific code.
//!
//! Currently only x86_64 is implemented.  The HAL traits in `hal/` provide
//! the abstraction layer for multi-architecture support.

pub mod x86_64;

/// Marker type for the x86_64 architecture (used by HAL trait impls).
pub struct X86_64;
