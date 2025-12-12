use crate::{
    addr::{PAddr, VAddr},
    frame_mgr::FrameMgr,
};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    pub const EMPTY: Self = Self(0);
}

impl PageTableEntry {
    pub const FLAG_VALID: usize = 1 << 0;
    pub const FLAG_READ: usize = 1 << 1;
    pub const FLAG_WRITE: usize = 1 << 2;
    pub const FLAG_EXECUTE: usize = 1 << 3;
    pub const FLAG_USER: usize = 1 << 4;
    pub const FLAG_GLOBAL: usize = 1 << 5;
    pub const FLAG_ACCESSED: usize = 1 << 6;
    pub const FLAG_DIRTY: usize = 1 << 7;

    pub fn new(paddr: PAddr, flags: usize) -> Self {
        PageTableEntry((paddr.ppn() << 10) | flags)
    }

    pub fn is_valid(&self) -> bool {
        (self.0 & Self::FLAG_VALID) != 0
    }

    pub fn paddr(&self) -> PAddr {
        PAddr::from_raw_addr((self.0 >> 10) << 12)
    }
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

impl PageTable {
    pub const EMPTY: Self = Self {
        entries: [PageTableEntry::EMPTY; 512],
    };
}

pub fn map(
    frame_mgr: &mut FrameMgr,
    page_table: &mut PageTable,
    vaddr: VAddr,
    paddr: PAddr,
    flags: usize,
) {
    // TODO: we must engage an lock here to prevent race conditions

    let ppn2 = vaddr.ppn2();
    let ppn1 = vaddr.ppn1();
    let ppn0 = vaddr.ppn0();

    if !page_table.entries[ppn2].is_valid() {
        let paddr = frame_mgr.alloc(1).expect("out of memory");
        let vaddr = paddr.to_hhdm();

        unsafe {
            core::ptr::write_bytes(vaddr.to_mut_ptr::<PageTable>(), 0, size_of::<PageTable>());
        }

        page_table.entries[ppn2] = PageTableEntry::new(paddr, PageTableEntry::FLAG_VALID);
    }

    // SAFE: we're converting physical address to virtual address
    let l1_addr = page_table.entries[ppn2].paddr().to_hhdm();
    let l1_ptr = l1_addr.to_mut_ptr::<PageTable>();
    let l1_table = unsafe { &mut *l1_ptr };

    if !l1_table.entries[ppn1].is_valid() {
        let paddr = frame_mgr.alloc(1).expect("out of memory");
        let vaddr = paddr.to_hhdm();

        unsafe {
            core::ptr::write_bytes(vaddr.to_mut_ptr::<PageTable>(), 0, size_of::<PageTable>());
        }

        l1_table.entries[ppn1] = PageTableEntry::new(paddr, PageTableEntry::FLAG_VALID);
    }

    let l0_addr = l1_table.entries[ppn1].paddr().to_hhdm();
    let l0_ptr = l0_addr.to_mut_ptr::<PageTable>();
    let l0_table = unsafe { &mut *l0_ptr };

    if l0_table.entries[ppn0].is_valid() {
        panic!(
            "page (vaddr={:x}, paddr={:x}) already mapped as paddr={:x}",
            vaddr.addr,
            paddr.addr,
            l0_table.entries[ppn0].paddr().addr
        );
    }

    l0_table.entries[ppn0] = PageTableEntry::new(paddr, PageTableEntry::FLAG_VALID | flags);
}

pub fn map_range(
    frame_mgr: &mut FrameMgr,
    page_table: &mut PageTable,
    vaddr: VAddr,
    paddr: PAddr,
    count: usize,
    flags: usize,
) {
    let mut current_vaddr = vaddr;
    let mut current_paddr = paddr;

    for _ in 0..count {
        map(frame_mgr, page_table, current_vaddr, current_paddr, flags);
        current_vaddr += FrameMgr::FRAME_SIZE;
        current_paddr += FrameMgr::FRAME_SIZE;
    }
}
