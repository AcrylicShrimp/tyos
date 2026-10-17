use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

pub fn read() -> Sie {
    let current: usize;

    unsafe {
        core::arch::asm!("csrr {}, sie", out(reg) current);
    }

    Sie(current)
}

pub unsafe fn set_flags(flags: SieFlags) {
    unsafe {
        core::arch::asm!("csrs sie, {}", in(reg) flags.0);
    }
}

pub unsafe fn clear_flags(flags: SieFlags) {
    unsafe {
        core::arch::asm!("csrc sie, {}", in(reg) flags.0);
    }
}

pub unsafe fn read_and_set_flags(flags: SieFlags) -> Sie {
    let old: usize;

    unsafe {
        core::arch::asm!("csrrs {}, sie, {}", out(reg) old, in(reg) flags.0);
    }

    Sie(old)
}

pub unsafe fn read_and_clear_flags(flags: SieFlags) -> Sie {
    let old: usize;

    unsafe {
        core::arch::asm!("csrrc {}, sie, {}", out(reg) old, in(reg) flags.0);
    }

    Sie(old)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sie(usize);

impl Sie {
    pub fn bits(self) -> usize {
        self.0
    }

    pub fn contains(self, flags: SieFlags) -> bool {
        self.0 & flags.0 == flags.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SieFlags(usize);

impl SieFlags {
    pub const EMPTY: Self = Self(0);

    /// Supervisor Software Interrupt Enable
    pub const SSIE: Self = Self(1 << 1);

    /// Supervisor Timer Interrupt Enable
    pub const STIE: Self = Self(1 << 5);

    /// Supervisor External Interrupt Enable
    pub const SEIE: Self = Self(1 << 9);

    pub fn bits(self) -> usize {
        self.0
    }
}

impl BitAnd for SieFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for SieFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for SieFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for SieFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
