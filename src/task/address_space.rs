use crate::{resources::memory::frame_alloc, vmem::PAGE_TABLE};
use alloc::vec::Vec;
use tyos_mem::{
    addr::{PAddr, VAddr},
    frame_mgr::FrameMgr,
    page_table::{MapError, MapOutcome, PageTable, PageTableEntry, UnmapError, map},
};

#[derive(Debug)]
pub enum TaskAddressSpaceError {
    OutOfMemory,
    ProgramEndNotAligned,
    ProgramEndOutOfRange,
    AddrOutOfRange,
    MapError(MapError),
    UnmapError(UnmapError),
}

impl From<TaskAddressSpaceBuilderError> for TaskAddressSpaceError {
    fn from(err: TaskAddressSpaceBuilderError) -> Self {
        match err {
            TaskAddressSpaceBuilderError::OutOfMemory => TaskAddressSpaceError::OutOfMemory,
            TaskAddressSpaceBuilderError::ProgramEndNotAligned => {
                TaskAddressSpaceError::ProgramEndNotAligned
            }
            TaskAddressSpaceBuilderError::ProgramEndOutOfRange => {
                TaskAddressSpaceError::ProgramEndOutOfRange
            }
            TaskAddressSpaceBuilderError::AddrOutOfRange => TaskAddressSpaceError::AddrOutOfRange,
            TaskAddressSpaceBuilderError::MapError(e) => TaskAddressSpaceError::MapError(e),
        }
    }
}

#[derive(Debug)]
pub struct TaskAddressSpace {
    root_page_frame: PAddr,

    user_heap_start: VAddr,
    user_heap_end: VAddr,

    user_main_thread_stack_top: VAddr,
    user_main_thread_stack_bottom: VAddr,

    kernel_stack_top: VAddr,
    kernel_stack_bottom: VAddr,

    owned_frames: Vec<PAddr>,
}

impl TaskAddressSpace {
    pub const LOWER_HALF_END: usize = 0x0000_0040_0000_0000;

    /// Kernel Stack: 16 KiB
    pub const KERNEL_STACK_FRAME_LEN: usize = 4;
    pub const KERNEL_STACK_SIZE: usize = Self::KERNEL_STACK_FRAME_LEN * FrameMgr::FRAME_SIZE;

    pub const KERNEL_STACK_GUARD_SIZE: usize = FrameMgr::FRAME_SIZE;

    pub const KERNEL_STACK_TOP: usize = Self::LOWER_HALF_END;
    pub const KERNEL_STACK_BOTTOM: usize = Self::KERNEL_STACK_TOP - Self::KERNEL_STACK_SIZE;

    /// Initial User Stack Size: 64 KiB
    pub const INITIAL_USER_STACK_FRAME_LEN: usize = 16;
    pub const INITIAL_USER_STACK_SIZE: usize =
        Self::INITIAL_USER_STACK_FRAME_LEN * FrameMgr::FRAME_SIZE;

    /// Maximum User Stack Size: 8MiB
    pub const MAX_USER_STACK_FRAME_LEN: usize = 2048;
    pub const MAX_USER_STACK_SIZE: usize = Self::MAX_USER_STACK_FRAME_LEN * FrameMgr::FRAME_SIZE;

    pub const USER_STACK_GUARD_SIZE: usize = FrameMgr::FRAME_SIZE;

    pub const USER_SPACE_END: usize = Self::KERNEL_STACK_BOTTOM - Self::KERNEL_STACK_GUARD_SIZE;
    pub const USER_SPACE_START: usize = 0x0000_0000_0000_1000;

    pub const USER_MAIN_THREAD_STACK_TOP: usize = Self::USER_SPACE_END;
    pub const USER_MAIN_THREAD_STACK_BOTTOM: usize =
        Self::USER_MAIN_THREAD_STACK_TOP - Self::MAX_USER_STACK_SIZE;

    pub const USER_HEAP_END: usize =
        Self::USER_MAIN_THREAD_STACK_BOTTOM - Self::USER_STACK_GUARD_SIZE;

    pub const USER_HEAP_GUARD_SIZE: usize = FrameMgr::FRAME_SIZE;

    pub const KERNEL_STACK_TOP_ADDR: VAddr = VAddr::from_raw_addr(Self::KERNEL_STACK_TOP);
    pub const KERNEL_STACK_BOTTOM_ADDR: VAddr = VAddr::from_raw_addr(Self::KERNEL_STACK_BOTTOM);

    pub const USER_SPACE_END_ADDR: VAddr = VAddr::from_raw_addr(Self::USER_SPACE_END);
    pub const USER_SPACE_START_ADDR: VAddr = VAddr::from_raw_addr(Self::USER_SPACE_START);

    pub const USER_MAIN_THREAD_STACK_TOP_ADDR: VAddr =
        VAddr::from_raw_addr(Self::USER_MAIN_THREAD_STACK_TOP);
    pub const USER_MAIN_THREAD_STACK_BOTTOM_ADDR: VAddr =
        VAddr::from_raw_addr(Self::USER_MAIN_THREAD_STACK_BOTTOM);

    pub const USER_HEAP_END_ADDR: VAddr = VAddr::from_raw_addr(Self::USER_HEAP_END);

    pub const fn program_vmem_range() -> core::ops::Range<VAddr> {
        let begin = Self::USER_SPACE_START;
        let end = Self::USER_HEAP_END - Self::USER_HEAP_GUARD_SIZE - FrameMgr::FRAME_SIZE;

        VAddr::from_raw_addr(begin)..VAddr::from_raw_addr(end)
    }
}

const _: () = {
    let range = TaskAddressSpace::program_vmem_range();

    assert!(
        range.start.addr < range.end.addr,
        "invalid program virtual memory range",
    );
};

impl TaskAddressSpace {
    /// ## SAFETY:
    ///
    /// Caller must guarantee this address space is not active on any hart.
    pub unsafe fn destroy(self) {
        frame_alloc().free_many(&self.owned_frames);
        frame_alloc().free_contiguous(self.root_page_frame, 1);
    }

    pub fn root_page_frame(&self) -> PAddr {
        self.root_page_frame
    }

    pub fn user_heap_range(&self) -> core::ops::Range<VAddr> {
        self.user_heap_start..self.user_heap_end
    }

    pub fn user_main_thread_stack_range(&self) -> core::ops::Range<VAddr> {
        self.user_main_thread_stack_bottom..self.user_main_thread_stack_top
    }

    pub fn kernel_stack_range(&self) -> core::ops::Range<VAddr> {
        self.kernel_stack_bottom..self.kernel_stack_top
    }
}

struct TaskAddressSpaceBuilderParts {
    root_page_frame: PAddr,

    user_heap_start: VAddr,
    user_heap_end: VAddr,

    user_main_thread_stack_top: VAddr,
    user_main_thread_stack_bottom: VAddr,

    kernel_stack_top: VAddr,
    kernel_stack_bottom: VAddr,

    owned_frames: Vec<PAddr>,
}

impl TaskAddressSpaceBuilderParts {
    fn page_table(&self) -> &PageTable {
        unsafe { &*self.root_page_frame.to_hhdm().to_ptr::<PageTable>() }
    }

    fn page_table_mut(&mut self) -> &mut PageTable {
        unsafe { &mut *self.root_page_frame.to_hhdm().to_mut_ptr::<PageTable>() }
    }

    pub unsafe fn destroy(self) {
        frame_alloc().free_many(&self.owned_frames);
        frame_alloc().free_contiguous(self.root_page_frame, 1);
    }
}

#[derive(Debug)]
pub enum TaskAddressSpaceBuilderError {
    OutOfMemory,
    ProgramEndNotAligned,
    ProgramEndOutOfRange,
    AddrOutOfRange,
    MapError(MapError),
}

pub struct TaskAddressSpaceBuilder {
    parts: Option<TaskAddressSpaceBuilderParts>,
}

impl TaskAddressSpaceBuilder {
    pub fn new(program_end: VAddr) -> Result<Self, TaskAddressSpaceBuilderError> {
        if !program_end.is_aligned(FrameMgr::FRAME_SIZE) {
            return Err(TaskAddressSpaceBuilderError::ProgramEndNotAligned);
        }

        if program_end < TaskAddressSpace::USER_SPACE_START_ADDR {
            return Err(TaskAddressSpaceBuilderError::ProgramEndOutOfRange);
        }

        if TaskAddressSpace::USER_HEAP_END_ADDR - TaskAddressSpace::USER_HEAP_GUARD_SIZE
            <= program_end
        {
            return Err(TaskAddressSpaceBuilderError::ProgramEndOutOfRange);
        }

        let root_page_frame = frame_alloc()
            .alloc_contiguous(1)
            .ok_or(TaskAddressSpaceBuilderError::OutOfMemory)?;

        let user_heap_start = program_end + TaskAddressSpace::USER_HEAP_GUARD_SIZE;
        let user_heap_end = user_heap_start;

        let user_main_thread_stack_top = TaskAddressSpace::USER_MAIN_THREAD_STACK_TOP_ADDR;
        let user_main_thread_stack_bottom =
            user_main_thread_stack_top - TaskAddressSpace::INITIAL_USER_STACK_SIZE;

        let kernel_stack_top = TaskAddressSpace::KERNEL_STACK_TOP_ADDR;
        let kernel_stack_bottom = TaskAddressSpace::KERNEL_STACK_BOTTOM_ADDR;

        Ok(Self {
            parts: Some(TaskAddressSpaceBuilderParts {
                root_page_frame,

                user_heap_start,
                user_heap_end,

                user_main_thread_stack_top,
                user_main_thread_stack_bottom,

                kernel_stack_top,
                kernel_stack_bottom,

                owned_frames: Vec::new(),
            }),
        })
    }

    pub fn setup_kernel_mappings(mut self) -> Self {
        /// TODO: do not expose the hhdm + kernel space for each task page map
        /// instead switch to the kernel page map when kernel traps
        fn build_task_page_table(page_table: &mut PageTable) {
            unsafe {
                // zero out the page table
                core::ptr::write_bytes::<PageTableEntry>(
                    page_table.entries.as_mut_ptr(),
                    0,
                    page_table.entries.len(),
                );

                // copy the shared page table to the task page table (hhdm + kernel space)
                core::ptr::copy_nonoverlapping::<PageTableEntry>(
                    PAGE_TABLE.entries[256..].as_ptr(),
                    page_table.entries[256..].as_mut_ptr(),
                    256,
                );
            }
        }

        build_task_page_table(self.parts.as_mut().unwrap().page_table_mut());
        self
    }

    pub fn setup_stack_frames(mut self) -> Result<Self, TaskAddressSpaceBuilderError> {
        let parts = self.parts.as_ref().unwrap();

        let user_stack_range =
            parts.user_main_thread_stack_bottom..parts.user_main_thread_stack_top;
        let kernel_stack_range = parts.kernel_stack_bottom..parts.kernel_stack_top;

        self.map_page_range_and_zero(
            user_stack_range.start,
            user_stack_range.end,
            PageTableEntry::FLAG_READ
                | PageTableEntry::FLAG_WRITE
                | PageTableEntry::FLAG_USER
                | PageTableEntry::FLAG_ACCESSED
                | PageTableEntry::FLAG_DIRTY,
        )?;
        self.map_page_range_and_zero(
            kernel_stack_range.start,
            kernel_stack_range.end,
            PageTableEntry::FLAG_READ
                | PageTableEntry::FLAG_WRITE
                | PageTableEntry::FLAG_ACCESSED
                | PageTableEntry::FLAG_DIRTY,
        )?;

        Ok(self)
    }

    pub fn map_zeroed_page(
        &mut self,
        vaddr: VAddr,
        flags: usize,
    ) -> Result<PAddr, TaskAddressSpaceBuilderError> {
        self.map_page_range_and_zero(vaddr, vaddr + FrameMgr::FRAME_SIZE, flags)
            .map(|frames| frames[0])
    }

    fn map_page_range_and_zero(
        &mut self,
        start: VAddr,
        end: VAddr,
        flags: usize,
    ) -> Result<Vec<PAddr>, TaskAddressSpaceBuilderError> {
        let frames = self.map_page_range(start, end, flags)?;

        for frame in &frames {
            unsafe {
                frame
                    .to_hhdm()
                    .to_mut_ptr::<u8>()
                    .write_bytes(0, FrameMgr::FRAME_SIZE);
            }
        }

        Ok(frames)
    }

    fn map_page_range(
        &mut self,
        start: VAddr,
        end: VAddr,
        flags: usize,
    ) -> Result<Vec<PAddr>, TaskAddressSpaceBuilderError> {
        let frame_count = (end - start) / FrameMgr::FRAME_SIZE;
        let parts = self.parts.as_mut().unwrap();

        parts
            .owned_frames
            .try_reserve(frame_count)
            .map_err(|_| TaskAddressSpaceBuilderError::OutOfMemory)?;

        let mut frames = Vec::new();

        frames
            .try_reserve_exact(frame_count)
            .map_err(|_| TaskAddressSpaceBuilderError::OutOfMemory)?;
        frames.resize(frame_count, PAddr::NULL);

        if !frame_alloc().alloc_many_into(&mut frames) {
            return Err(TaskAddressSpaceBuilderError::OutOfMemory);
        }

        parts.owned_frames.extend_from_slice(&frames);

        let compute_vaddr = |frame_index: usize| start + frame_index * FrameMgr::FRAME_SIZE;

        for (frame_index, frame) in frames.iter().cloned().enumerate() {
            self.map_page(compute_vaddr(frame_index), frame, flags)?;
        }

        Ok(frames)
    }

    fn map_page(
        &mut self,
        vaddr: VAddr,
        paddr: PAddr,
        flags: usize,
    ) -> Result<(), TaskAddressSpaceBuilderError> {
        let parts = self.parts.as_mut().unwrap();

        parts
            .owned_frames
            .try_reserve(2)
            .map_err(|_| TaskAddressSpaceBuilderError::OutOfMemory)?;

        let outcome = map(frame_alloc(), parts.page_table_mut(), vaddr, paddr, flags);

        let MapOutcome {
            allocated_frames: [addr2, addr1],
        } = outcome.map_err(TaskAddressSpaceBuilderError::MapError)?;

        if addr2 != PAddr::NULL {
            parts.owned_frames.push(addr2);
        }

        if addr1 != PAddr::NULL {
            parts.owned_frames.push(addr1);
        }

        Ok(())
    }

    pub fn commit(mut self) -> TaskAddressSpace {
        let parts = self.parts.take().unwrap();

        TaskAddressSpace {
            root_page_frame: parts.root_page_frame,

            user_heap_start: parts.user_heap_start,
            user_heap_end: parts.user_heap_end,

            user_main_thread_stack_top: parts.user_main_thread_stack_top,
            user_main_thread_stack_bottom: parts.user_main_thread_stack_bottom,

            kernel_stack_top: parts.kernel_stack_top,
            kernel_stack_bottom: parts.kernel_stack_bottom,

            owned_frames: parts.owned_frames,
        }
    }
}

impl Drop for TaskAddressSpaceBuilder {
    fn drop(&mut self) {
        if let Some(parts) = self.parts.take() {
            unsafe {
                parts.destroy();
            }
        }
    }
}
