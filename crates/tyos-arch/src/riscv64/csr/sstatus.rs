use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

pub fn read() -> Sstatus {
    let current: usize;

    unsafe {
        core::arch::asm!("csrr {}, sstatus", out(reg) current);
    }

    Sstatus(current)
}

pub unsafe fn set_flags(flags: SstatusFlags) {
    unsafe {
        core::arch::asm!("csrs sstatus, {}", in(reg) flags.0);
    }
}

pub unsafe fn clear_flags(flags: SstatusFlags) {
    unsafe {
        core::arch::asm!("csrc sstatus, {}", in(reg) flags.0);
    }
}

pub unsafe fn read_and_set_flags(flags: SstatusFlags) -> Sstatus {
    let old: usize;

    unsafe {
        core::arch::asm!("csrrs {}, sstatus, {}", out(reg) old, in(reg) flags.0);
    }

    Sstatus(old)
}

pub unsafe fn read_and_clear_flags(flags: SstatusFlags) -> Sstatus {
    let old: usize;

    unsafe {
        core::arch::asm!("csrrc {}, sstatus, {}", out(reg) old, in(reg) flags.0);
    }

    Sstatus(old)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sstatus(usize);

impl Sstatus {
    pub fn bits(self) -> usize {
        self.0
    }

    pub fn contains(self, flags: SstatusFlags) -> bool {
        self.0 & flags.0 == flags.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SstatusFlags(usize);

impl SstatusFlags {
    pub const EMPTY: Self = Self(0);

    /// Supervisor Interrupt Enable
    pub const SIE: Self = Self(1 << 1);

    /// Supervisor Previous Interrupt Enable
    pub const SPIE: Self = Self(1 << 5);

    /// Supervisor Previous Privilege Level
    pub const SPP: Self = Self(1 << 8);

    pub fn bits(self) -> usize {
        self.0
    }
}

impl BitAnd for SstatusFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for SstatusFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for SstatusFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for SstatusFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
