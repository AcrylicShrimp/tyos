#![no_std]

mod syscall;
mod syscall_id;
mod syscalls;

pub use syscall_id::*;
pub use syscalls::*;
