use crate::{
    addr::{PAddr, VAddr},
    frame_alloc::FrameAlloc,
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
    pub fn zero_out(&mut self) {
        unsafe {
            core::ptr::write_bytes(self.entries.as_mut_ptr(), 0, self.entries.len());
        }
    }
}

impl PageTable {
    pub const EMPTY: Self = Self {
        entries: [PageTableEntry::EMPTY; 512],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapOutcome {
    pub allocated_frames: [PAddr; 2],
}

#[derive(Debug)]
pub enum MapError {
    AddressIsNull,
    AddressNotAligned,
    OutOfMemory,
    AlreadyMapped,
}

pub fn map(
    frame_alloc: &FrameAlloc,
    page_table: &mut PageTable,
    vaddr: VAddr,
    paddr: PAddr,
    flags: usize,
) -> Result<MapOutcome, MapError> {
    if vaddr == VAddr::NULL {
        return Err(MapError::AddressIsNull);
    }

    if !vaddr.is_aligned(FrameMgr::FRAME_SIZE) {
        return Err(MapError::AddressNotAligned);
    }

    if paddr == PAddr::NULL {
        return Err(MapError::AddressIsNull);
    }

    if !paddr.is_aligned(FrameMgr::FRAME_SIZE) {
        return Err(MapError::AddressNotAligned);
    }

    let mut allocated_frames = [PAddr::NULL; 2];

    fn rollback(frame_alloc: &FrameAlloc, [addr2, addr1]: [PAddr; 2]) {
        if addr2 != PAddr::NULL {
            frame_alloc.free_contiguous(addr2, 1);
        }

        if addr1 != PAddr::NULL {
            frame_alloc.free_contiguous(addr1, 1);
        }
    }

    let ppn2 = vaddr.ppn2();
    let ppn1 = vaddr.ppn1();
    let ppn0 = vaddr.ppn0();

    let (is_l1_allocated, l1_addr) = if page_table.entries[ppn2].is_valid() {
        (false, page_table.entries[ppn2].paddr())
    } else {
        let paddr = match frame_alloc.alloc_contiguous(1) {
            Some(paddr) => {
                allocated_frames[0] = paddr;
                paddr
            }
            None => {
                rollback(frame_alloc, allocated_frames);
                return Err(MapError::OutOfMemory);
            }
        };
        let vaddr = paddr.to_hhdm();

        unsafe {
            let page_table = &mut *vaddr.to_mut_ptr::<PageTable>();
            page_table.zero_out();
        }

        (true, paddr)
    };
    let l1_ptr = l1_addr.to_hhdm().to_mut_ptr::<PageTable>();
    let l1_table = unsafe { &mut *l1_ptr };

    let (is_l0_allocated, l0_addr) = if l1_table.entries[ppn1].is_valid() {
        (false, l1_table.entries[ppn1].paddr())
    } else {
        let paddr = match frame_alloc.alloc_contiguous(1) {
            Some(paddr) => {
                allocated_frames[1] = paddr;
                paddr
            }
            None => {
                rollback(frame_alloc, allocated_frames);
                return Err(MapError::OutOfMemory);
            }
        };
        let vaddr = paddr.to_hhdm();

        unsafe {
            let page_table = &mut *vaddr.to_mut_ptr::<PageTable>();
            page_table.zero_out();
        }

        (true, paddr)
    };
    let l0_ptr = l0_addr.to_hhdm().to_mut_ptr::<PageTable>();
    let l0_table = unsafe { &mut *l0_ptr };

    if l0_table.entries[ppn0].is_valid() {
        rollback(frame_alloc, allocated_frames);
        return Err(MapError::AlreadyMapped);
    }

    l0_table.entries[ppn0] = PageTableEntry::new(paddr, PageTableEntry::FLAG_VALID | flags);

    if is_l0_allocated {
        // Mark the L0 page table as valid in the L1 table
        l1_table.entries[ppn1] = PageTableEntry::new(l0_addr, PageTableEntry::FLAG_VALID);
    }

    if is_l1_allocated {
        // Mark the L1 page table as valid in the L2 table
        page_table.entries[ppn2] = PageTableEntry::new(l1_addr, PageTableEntry::FLAG_VALID);
    }

    unsafe {
        core::arch::asm!("sfence.vma");
    }

    Ok(MapOutcome { allocated_frames })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnmapOutcome {
    pub paddr: PAddr,
    pub removed_frames: [PAddr; 2],
}

#[derive(Debug)]
pub enum UnmapError {
    AddressIsNull,
    AddressNotAligned,
    NotMapped,
}

pub unsafe fn unmap(page_table: &mut PageTable, vaddr: VAddr) -> Result<UnmapOutcome, UnmapError> {
    if vaddr == VAddr::NULL {
        return Err(UnmapError::AddressIsNull);
    }

    if !vaddr.is_aligned(FrameMgr::FRAME_SIZE) {
        return Err(UnmapError::AddressNotAligned);
    }

    let ppn2 = vaddr.ppn2();
    let ppn1 = vaddr.ppn1();
    let ppn0 = vaddr.ppn0();

    if !page_table.entries[ppn2].is_valid() {
        return Err(UnmapError::NotMapped);
    }

    let l1_addr = page_table.entries[ppn2].paddr().to_hhdm();
    let l1_ptr = l1_addr.to_mut_ptr::<PageTable>();
    let l1_table = unsafe { &mut *l1_ptr };

    if !l1_table.entries[ppn1].is_valid() {
        return Err(UnmapError::NotMapped);
    }

    let l0_addr = l1_table.entries[ppn1].paddr().to_hhdm();
    let l0_ptr = l0_addr.to_mut_ptr::<PageTable>();
    let l0_table = unsafe { &mut *l0_ptr };

    if !l0_table.entries[ppn0].is_valid() {
        return Err(UnmapError::NotMapped);
    }

    let paddr = l0_table.entries[ppn0].paddr();

    l0_table.entries[ppn0] = PageTableEntry::EMPTY;

    let mut removed_frames = [PAddr::NULL; 2];

    let is_l0_empty = l0_table.entries.iter().all(|entry| !entry.is_valid());

    if is_l0_empty {
        removed_frames[0] = l1_table.entries[ppn1].paddr();
        l1_table.entries[ppn1] = PageTableEntry::EMPTY;
    }

    let is_l1_empty = l1_table.entries.iter().all(|entry| !entry.is_valid());

    if is_l1_empty {
        removed_frames[1] = page_table.entries[ppn2].paddr();
        page_table.entries[ppn2] = PageTableEntry::EMPTY;
    }

    Ok(UnmapOutcome {
        paddr,
        removed_frames,
    })
}

#[derive(Debug)]
pub enum ReadMappingError {
    AddressIsNull,
    AddressNotAligned,
    NotMapped,
}

pub fn read_mapping(page_table: &PageTable, vaddr: VAddr) -> Result<PAddr, ReadMappingError> {
    if vaddr == VAddr::NULL {
        return Err(ReadMappingError::AddressIsNull);
    }

    if !vaddr.is_aligned(FrameMgr::FRAME_SIZE) {
        return Err(ReadMappingError::AddressNotAligned);
    }

    let ppn2 = vaddr.ppn2();
    let ppn1 = vaddr.ppn1();
    let ppn0 = vaddr.ppn0();

    if !page_table.entries[ppn2].is_valid() {
        return Err(ReadMappingError::NotMapped);
    }

    let l1_addr = page_table.entries[ppn2].paddr().to_hhdm();
    let l1_ptr = l1_addr.to_mut_ptr::<PageTable>();
    let l1_table = unsafe { &mut *l1_ptr };

    if !l1_table.entries[ppn1].is_valid() {
        return Err(ReadMappingError::NotMapped);
    }

    let l0_addr = l1_table.entries[ppn1].paddr().to_hhdm();
    let l0_ptr = l0_addr.to_mut_ptr::<PageTable>();
    let l0_table = unsafe { &mut *l0_ptr };

    if !l0_table.entries[ppn0].is_valid() {
        return Err(ReadMappingError::NotMapped);
    }

    Ok(l0_table.entries[ppn0].paddr())
}
