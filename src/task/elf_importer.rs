use alloc::vec::Vec;
use core::ops::Range;
use goblin::elf::{
    Elf,
    program_header::{self, ProgramHeader},
};
use tyos_mem::{addr::VAddr, frame_mgr::FrameMgr, page_table::PageTableEntry};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutConfig {
    pub vmem_range: Range<VAddr>,
}

#[derive(Debug)]
pub enum LayoutElfError {
    InvalidElfFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgramLayout {
    pub frame_map_plans: Vec<ProgramFrameMapPlan>,
    pub copy_plans: Vec<ProgramCopyPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgramFrameMapPlan {
    pub aligned_addr: VAddr,
    pub flags: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgramCopyPlan {
    pub len: usize,
    pub src_offset: usize,
    pub dest_frame_index: usize,
    pub dest_offset: usize,
}

fn is_valid_header(header: &ProgramHeader, file_size: usize, config: &LayoutConfig) -> bool {
    let vm_range = addr_range(header.vm_range());

    if vm_range.start >= vm_range.end {
        return false;
    }

    let aligned_vm_range = align_vaddr_range(vm_range.clone());

    if aligned_vm_range.start < config.vmem_range.start {
        return false;
    }

    if config.vmem_range.end < aligned_vm_range.end {
        return false;
    }

    let file_range = header.file_range();

    if file_range.start > file_range.end {
        return false;
    }

    if file_size < file_range.end {
        return false;
    }

    if vaddr_range_len(vm_range.clone()) < file_range.len() {
        return false;
    }

    true
}

pub fn layout_elf(
    elf: &Elf,
    elf_file_size: usize,
    config: &LayoutConfig,
) -> Result<ProgramLayout, LayoutElfError> {
    ///////////////////////////////////////////
    // Phase 0: Sanity Check
    ///////////////////////////////////////////

    let headers = elf
        .program_headers
        .iter()
        .filter(|header| header.p_type == program_header::PT_LOAD);

    if headers
        .clone()
        .any(|header| !is_valid_header(header, elf_file_size, config))
    {
        return Err(LayoutElfError::InvalidElfFile);
    }

    ///////////////////////////////////////////
    // Phase 1: Frame Map Plans
    ///////////////////////////////////////////

    let mut frame_map_plans: Vec<ProgramFrameMapPlan> = Vec::new();
    let mut copy_plans: Vec<ProgramCopyPlan> = Vec::new();

    for header in headers.clone() {
        let vm_range = addr_range(header.vm_range());
        let aligned_vm_range = align_vaddr_range(vm_range.clone());
        let header_flags = header_flags_to_page_table_flags(header.p_flags);

        for offset in (0..vaddr_range_len(aligned_vm_range.clone())).step_by(FrameMgr::FRAME_SIZE) {
            let vaddr = aligned_vm_range.start + offset;

            match frame_map_plans.binary_search_by_key(&vaddr, |plan| plan.aligned_addr) {
                Ok(index) => {
                    frame_map_plans[index].flags |= header_flags;
                }
                Err(index) => {
                    frame_map_plans.insert(
                        index,
                        ProgramFrameMapPlan {
                            aligned_addr: vaddr,
                            flags: header_flags,
                        },
                    );
                }
            };
        }
    }

    ///////////////////////////////////////////
    // Phase 2: Copy Plans
    ///////////////////////////////////////////

    let mut copy_ranges: Vec<Range<VAddr>> = Vec::new();

    for header in headers {
        let vm_range = addr_range(header.vm_range());
        let file_range = header.file_range();

        if file_range.is_empty() {
            continue;
        }

        let copy_range = vm_range.start..(vm_range.start + file_range.len());

        if copy_ranges
            .iter()
            .any(|lhs| ranges_overlap(lhs, &copy_range))
        {
            return Err(LayoutElfError::InvalidElfFile);
        }

        copy_ranges.push(copy_range.clone());

        let aligned_copy_range = align_vaddr_range(copy_range.clone());

        for offset in (0..vaddr_range_len(aligned_copy_range.clone())).step_by(FrameMgr::FRAME_SIZE)
        {
            let frame_range = (aligned_copy_range.start + offset)
                ..(aligned_copy_range.start + offset + FrameMgr::FRAME_SIZE);
            let intersection =
                frame_range.start.max(copy_range.start)..frame_range.end.min(copy_range.end);

            let len = vaddr_range_len(intersection.clone());
            let src_offset = file_range.start + intersection.start - copy_range.start;
            let dest_frame_index = match frame_map_plans
                .binary_search_by_key(&frame_range.start, |plan| plan.aligned_addr)
            {
                Ok(index) => index,
                Err(_) => {
                    unreachable!("[FATAL] unable to find destination frame index to be copied");
                }
            };
            let dest_offset = intersection.start - aligned_copy_range.start - offset;

            copy_plans.push(ProgramCopyPlan {
                len,
                src_offset,
                dest_frame_index,
                dest_offset,
            });
        }
    }

    Ok(ProgramLayout {
        frame_map_plans,
        copy_plans,
    })
}

#[derive(Debug)]
pub enum PrepareElfError {
    InvalidElfFile,
    OutOfMemory,
}

impl From<LayoutElfError> for PrepareElfError {
    fn from(value: LayoutElfError) -> Self {
        match value {
            LayoutElfError::InvalidElfFile => Self::InvalidElfFile,
        }
    }
}

// pub struct PreparedProgram {
//     pub page_table: PageTable,
//     pub allocated_frames: Vec<PAddr>,
// }

// impl Drop for PreparedProgram {
//     fn drop(&mut self) {
//         get_frame_alloc().free_many(&self.allocated_frames);
//     }
// }

// pub fn prepare_elf(elf: &Elf, elf_file: &[u8]) -> Result<PreparedProgram, PrepareElfError> {
//     let layout = layout_elf(elf, elf_file.len())?;
//     let mut frames = vec![PAddr::NULL; layout.frame_map_plans.len()];

//     if !get_frame_alloc().alloc_many_into(frames.as_mut_slice()) {
//         return Err(PrepareElfError::OutOfMemory);
//     }

//     for frame in &frames {
//         unsafe {
//             core::ptr::write_bytes(frame.to_hhdm().to_mut_ptr::<u8>(), 0, FrameMgr::FRAME_SIZE);
//         }
//     }

//     let mut page_table = PageTable::EMPTY;

//     for (frame, plan) in frames.iter().zip(layout.frame_map_plans.iter()) {
//         map(
//             get_frame_alloc(),
//             &mut page_table,
//             plan.aligned_addr,
//             *frame,
//             plan.flags | PageTableEntry::FLAG_USER,
//         );
//     }

//     for plan in &layout.copy_plans {
//         unsafe {
//             (frames[plan.dest_frame_index] + plan.dest_offset)
//                 .to_hhdm()
//                 .to_mut_ptr::<u8>()
//                 .copy_from_nonoverlapping(elf_file[plan.src_offset..].as_ptr(), plan.len);
//         }
//     }

//     Ok(PreparedProgram {
//         page_table,
//         allocated_frames: frames,
//     })
// }

fn addr_range(range: core::ops::Range<usize>) -> core::ops::Range<VAddr> {
    VAddr::from_raw_addr(range.start)..VAddr::from_raw_addr(range.end)
}

fn vaddr_range_len(range: core::ops::Range<VAddr>) -> usize {
    range.end.addr - range.start.addr
}

fn align_vaddr_range(range: core::ops::Range<VAddr>) -> core::ops::Range<VAddr> {
    let start = range.start.align_down(FrameMgr::FRAME_SIZE);
    let end = range.end.align_up(FrameMgr::FRAME_SIZE);
    start..end
}

fn header_flags_to_page_table_flags(flags: u32) -> usize {
    let mut result = 0;

    if flags & program_header::PF_X != 0 {
        result |= PageTableEntry::FLAG_EXECUTE | PageTableEntry::FLAG_ACCESSED;
    }

    if flags & program_header::PF_W != 0 {
        result |= PageTableEntry::FLAG_READ
            | PageTableEntry::FLAG_WRITE
            | PageTableEntry::FLAG_ACCESSED
            | PageTableEntry::FLAG_DIRTY;
    }

    if flags & program_header::PF_R != 0 {
        result |= PageTableEntry::FLAG_READ | PageTableEntry::FLAG_ACCESSED;
    }

    result
}

fn ranges_overlap<Idx: Copy + Ord>(lhs: &Range<Idx>, rhs: &Range<Idx>) -> bool {
    Idx::max(lhs.start, rhs.start) < Idx::min(lhs.end, rhs.end)
}
