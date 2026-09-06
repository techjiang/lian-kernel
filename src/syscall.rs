//! System call interface.
//!
//! Userspace programs invoke syscalls via `int 0x80`.  The syscall number
//! is passed in `rax`, and up to 6 arguments in `rdi, rsi, rdx, r10, r8, r9`.

use crate::arch::x86_64::idt::InterruptFrame;

/// Syscall numbers.
pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_READ: u64 = 2;
pub const SYS_YIELD: u64 = 3;
pub const SYS_GETPID: u64 = 4;
pub const SYS_OPEN: u64 = 5;
pub const SYS_CLOSE: u64 = 6;
pub const SYS_SPAWN: u64 = 7;
pub const SYS_DUMP: u64 = 8;
pub const SYS_MEMINFO: u64 = 9;

/// Handle a syscall.  Called from the `dispatch` function for vector 128.
///
/// The frame's `rax` contains the syscall number; `rdi..r9` contain args.
pub fn handle_syscall(frame: *const InterruptFrame) {
    let f = unsafe { &*frame };

    let num = f.rax;
    let arg0 = f.rdi;
    let arg1 = f.rsi;
    let _arg2 = f.rdx;

    match num {
        SYS_WRITE => {
            // arg0 = fd, arg1 = pointer to string, arg2 = length (in rdx)
            // For simplicity, treat fd=1 as serial output.
            if arg0 == 1 {
                let buf = unsafe { core::slice::from_raw_parts(arg1 as *const u8, _arg2 as usize) };
                if let Ok(s) = core::str::from_utf8(buf) {
                    crate::serial_println!("[syscall:write] {}", s);
                    crate::print!("{}", s);
                }
            }
        }
        SYS_YIELD => {
            crate::println!("[syscall:yield]");
            crate::process::scheduler::on_timer_tick();
        }
        SYS_GETPID => {
            crate::serial_println!("[syscall:getpid]");
        }
        SYS_DUMP => {
            crate::process::scheduler::dump_tasks();
            crate::println!("[syscall:dump] Free frames: {}", crate::memory::frame::free_frames());
        }
        SYS_MEMINFO => {
            let free = crate::memory::frame::free_frames();
            crate::println!("[syscall:meminfo] Free: {} frames ({} MiB)", free, free * 4096 / (1024 * 1024));
        }
        SYS_EXIT => {
            crate::println!("[syscall:exit] code={}", arg0);
        }
        _ => {
            crate::println!("[syscall] unknown syscall #{}", num);
        }
    }
}
