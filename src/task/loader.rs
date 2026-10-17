use crate::task::{
    address_space::{
        TaskAddressSpace, TaskAddressSpaceBuilder, TaskAddressSpaceBuilderError,
        TaskAddressSpaceError,
    },
    program::{ProgramDef, ProgramDefSectionPermission},
};
use alloc::vec::Vec;
use tyos_mem::{addr::VAddr, frame_mgr::FrameMgr, page_table::PageTableEntry};

#[derive(Debug)]
pub enum ProgramLoadError {
    OutOfMemory,
    EndNotDefined,
    EndOverflow,
    SectionRangeOverflow,
    SourceRangeOverflow,
    SectionNotFound,
    InvalidDataLen,
    TaskAddressSpaceError(TaskAddressSpaceError),
    TaskAddressSpaceBuilderError(TaskAddressSpaceBuilderError),
}

impl From<TaskAddressSpaceError> for ProgramLoadError {
    fn from(err: TaskAddressSpaceError) -> Self {
        ProgramLoadError::TaskAddressSpaceError(err)
    }
}

impl From<TaskAddressSpaceBuilderError> for ProgramLoadError {
    fn from(err: TaskAddressSpaceBuilderError) -> Self {
        ProgramLoadError::TaskAddressSpaceBuilderError(err)
    }
}

#[derive(Debug)]
pub struct LoadedProgram {
    pub entry: VAddr,
    pub aligned_end: VAddr,
    pub addr_space: TaskAddressSpace,
}

pub fn load_program(def: &ProgramDef, data: &[u8]) -> Result<LoadedProgram, ProgramLoadError> {
    let load_plan = resolve_load_plan(def)?;

    let end = def.end().ok_or(ProgramLoadError::EndNotDefined)?;
    let aligned_end = end
        .checked_align_up(FrameMgr::FRAME_SIZE)
        .ok_or(ProgramLoadError::EndOverflow)?;

    let mut addr_space_builder = TaskAddressSpaceBuilder::new(aligned_end)?
        .setup_kernel_mappings()
        .setup_stack_frames()?;

    for section in &load_plan {
        let frame = addr_space_builder
            .map_zeroed_page(section.vaddr, permission_to_page_flags(section.permission))?;

        for source in &section.sources {
            let dest = frame + source.dest_local_offset;

            let src_start = source.src_offset;
            let src_end = source
                .src_offset
                .checked_add(source.len)
                .ok_or(ProgramLoadError::SourceRangeOverflow)?;

            let src = data
                .get(src_start..src_end)
                .ok_or(ProgramLoadError::InvalidDataLen)?;

            unsafe {
                core::ptr::copy_nonoverlapping(
                    src.as_ptr(),
                    dest.to_hhdm().to_mut_ptr::<u8>(),
                    source.len,
                );
            }
        }
    }

    Ok(LoadedProgram {
        entry: def.entry(),
        aligned_end,
        addr_space: addr_space_builder.commit(),
    })
}

fn permission_to_page_flags(permission: ProgramDefSectionPermission) -> usize {
    let mut flags = 0;

    let is_read = permission.contains(ProgramDefSectionPermission::READ);
    let is_write = permission.contains(ProgramDefSectionPermission::WRITE);
    let is_exec = permission.contains(ProgramDefSectionPermission::EXEC);

    if is_read {
        flags |= PageTableEntry::FLAG_READ;
    }

    if is_write {
        flags |=
            PageTableEntry::FLAG_READ | PageTableEntry::FLAG_WRITE | PageTableEntry::FLAG_DIRTY;
    }

    if is_exec {
        flags |= PageTableEntry::FLAG_EXECUTE;
    }

    flags | PageTableEntry::FLAG_USER | PageTableEntry::FLAG_ACCESSED
}

struct AlignedSection {
    vaddr: VAddr,
    permission: ProgramDefSectionPermission,
    sources: Vec<AlignedSectionSource>,
}

struct AlignedSectionSource {
    dest_local_offset: usize,
    src_offset: usize,
    len: usize,
}

fn resolve_load_plan(def: &ProgramDef) -> Result<Vec<AlignedSection>, ProgramLoadError> {
    let mut aligned_sections: Vec<AlignedSection> = Vec::new();

    for section in def.sections() {
        let addr_start = section.start().checked_align_down(FrameMgr::FRAME_SIZE);
        let addr_end = section
            .end()
            .and_then(|end| end.checked_align_up(FrameMgr::FRAME_SIZE));

        let (addr_start, addr_end) = match (addr_start, addr_end) {
            (Some(start), Some(end)) => (start, end),
            _ => {
                return Err(ProgramLoadError::SectionRangeOverflow);
            }
        };

        for offset in (0..(addr_end - addr_start)).step_by(FrameMgr::FRAME_SIZE) {
            let addr = addr_start + offset;

            match aligned_sections.binary_search_by_key(&addr, |section| section.vaddr) {
                Ok(index) => {
                    aligned_sections[index].permission |= section.permission;
                }
                Err(index) => {
                    aligned_sections
                        .try_reserve(1)
                        .map_err(|_| ProgramLoadError::OutOfMemory)?;

                    aligned_sections.insert(
                        index,
                        AlignedSection {
                            vaddr: addr,
                            permission: section.permission,
                            sources: Vec::new(),
                        },
                    );
                }
            }
        }
    }

    for source in def.sources() {
        let dest_start = source.dest_start();
        let dest_end = source
            .dest_end()
            .ok_or(ProgramLoadError::SourceRangeOverflow)?;

        let aligned_dest_start = dest_start
            .checked_align_down(FrameMgr::FRAME_SIZE)
            .ok_or(ProgramLoadError::SourceRangeOverflow)?;
        let aligned_dest_end = dest_end
            .checked_align_up(FrameMgr::FRAME_SIZE)
            .ok_or(ProgramLoadError::SourceRangeOverflow)?;

        for offset in (0..(aligned_dest_end - aligned_dest_start)).step_by(FrameMgr::FRAME_SIZE) {
            let dest_section_start = aligned_dest_start + offset;
            let dest_section_end = dest_section_start + FrameMgr::FRAME_SIZE;

            let section_index = aligned_sections
                .binary_search_by_key(&dest_section_start, |section| section.vaddr)
                .map_err(|_| ProgramLoadError::SectionNotFound)?;

            let intersection_start = dest_section_start.max(dest_start);
            let intersection_end = dest_section_end.min(dest_end);

            let dest_local_offset = intersection_start - dest_section_start;
            let src_offset = (intersection_start - dest_start)
                .checked_add(source.src_offset)
                .ok_or(ProgramLoadError::SourceRangeOverflow)?;
            let len = intersection_end - intersection_start;

            let section = &mut aligned_sections[section_index];

            section
                .sources
                .try_reserve(1)
                .map_err(|_| ProgramLoadError::OutOfMemory)?;

            section.sources.push(AlignedSectionSource {
                dest_local_offset,
                src_offset,
                len,
            });
        }
    }

    Ok(aligned_sections)
}
