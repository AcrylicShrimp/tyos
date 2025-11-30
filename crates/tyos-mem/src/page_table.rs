#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    pub const FLAG_VALID: usize = 1 << 0;
    pub const FLAG_READ: usize = 1 << 1;
    pub const FLAG_WRITE: usize = 1 << 2;
    pub const FLAG_EXECUTE: usize = 1 << 3;
    pub const FLAG_USER: usize = 1 << 4;
    pub const FLAG_GLOBAL: usize = 1 << 5;
    pub const FLAG_ACCESSED: usize = 1 << 6;
    pub const FLAG_DIRTY: usize = 1 << 7;

    pub fn new(phsc_addr: usize, flags: usize) -> Self {
        let ppn = (phsc_addr >> 12) & 0x0000_03FF_FFFF_FFFF;
        PageTableEntry((ppn << 10) | flags)
    }

    pub fn is_valid(&self) -> bool {
        (self.0 & Self::FLAG_VALID) != 0
    }

    pub fn phys_addr(&self) -> usize {
        (self.0 >> 10) << 12
    }
}
