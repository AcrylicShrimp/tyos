#![no_std]

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64::{};

#[cfg(target_arch = "riscv64")]
pub mod raw {
    pub use super::riscv64::*;
}
