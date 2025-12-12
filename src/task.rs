use crate::{hal::get_frame_mgr, vmem::build_task_page_table};
use alloc::vec::Vec;
use goblin::elf::{Elf, program_header};
use tyos_mem::{
    addr::{PAddr, VAddr},
    frame_mgr::FrameMgr,
    page_table::{PageTable, PageTableEntry, map_range},
    page_table_config::{PageTableConfig, PageTableMode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Exited,
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct TaskAllocatedFrame {
    pub addr: PAddr,
    pub count: usize,
}

impl TaskAllocatedFrame {
    pub const fn new(addr: PAddr, count: usize) -> Self {
        Self { addr, count }
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct TaskMemRange {
    pub bottom: VAddr,
    pub top: VAddr,
}

impl TaskMemRange {
    pub const EMPTY: Self = Self {
        bottom: VAddr::NULL,
        top: VAddr::NULL,
    };

    pub const fn new(bottom: VAddr, top: VAddr) -> Self {
        Self { bottom, top }
    }
}

#[derive(Debug)]
pub struct Task {
    pid: usize,
    state: TaskState,
    context: TaskContext,
    page_table_config: PageTableConfig,
    kernel_stack: TaskMemRange,
    user_stack: TaskMemRange,
    user_heap: TaskMemRange,
    allocated_frames: Vec<TaskAllocatedFrame>,
}

impl Task {
    const USER_TOP_0: VAddr = VAddr::from_raw_addr(0x0000_003F_FFFF_F000);
    const USER_TOP_1: VAddr = VAddr::from_raw_addr(0x0000_003F_FFF0_0000);
    const USER_BOTTOM: VAddr = VAddr::from_raw_addr(0x0000_0000_0000_1000);

    pub fn empty(pid: usize) -> Self {
        Self {
            pid,
            state: TaskState::Ready,
            context: TaskContext::EMPTY,
            page_table_config: PageTableConfig::new(PageTableMode::Sv39, 0, 0),
            kernel_stack: TaskMemRange::EMPTY,
            user_stack: TaskMemRange::EMPTY,
            user_heap: TaskMemRange::EMPTY,
            allocated_frames: Vec::new(),
        }
    }

    pub fn load_from_elf(pid: usize, elf: &Elf, elf_file: &[u8]) -> Self {
        let mut allocated_frames = Vec::new();

        let page_table_paddr = { get_frame_mgr().alloc(1) }.expect("out of memory");
        let page_table_config =
            PageTableConfig::new(PageTableMode::Sv39, 0, page_table_paddr.ppn());
        allocated_frames.push(TaskAllocatedFrame::new(page_table_paddr, 1));

        let page_table = unsafe { &mut *page_table_paddr.to_hhdm().to_mut_ptr::<PageTable>() };
        build_task_page_table(page_table);

        let mut program_end_vaddr = Self::USER_BOTTOM.addr;

        // Program structures
        for header in &elf.program_headers {
            if header.p_type != program_header::PT_LOAD {
                continue;
            }

            let unaligned_vm_range = header.vm_range();
            let vm_range = VAddr::from_raw_addr(unaligned_vm_range.start)
                .align_down(FrameMgr::FRAME_SIZE)
                .addr
                ..VAddr::from_raw_addr(unaligned_vm_range.end)
                    .align_up(FrameMgr::FRAME_SIZE)
                    .addr;

            program_end_vaddr = program_end_vaddr.max(vm_range.end);

            if vm_range.start < Self::USER_BOTTOM.addr {
                panic!(
                    "program section is too low (asserted: {:x} <= {:x})",
                    Self::USER_BOTTOM.addr,
                    vm_range.start,
                );
            }

            if Self::USER_TOP_1.addr < vm_range.end {
                panic!(
                    "program section is too high (asserted: {:x} <= {:x})",
                    vm_range.end,
                    Self::USER_TOP_1.addr,
                );
            }

            let file_range = header.file_range();

            if elf_file.len() < file_range.end {
                panic!("program section is too large for elf file");
            }

            let frame_count = vm_range.len() / FrameMgr::FRAME_SIZE;
            let frame_paddr = { get_frame_mgr().alloc(frame_count) }.expect("out of memory");
            allocated_frames.push(TaskAllocatedFrame::new(frame_paddr, frame_count));

            let mut flags = PageTableEntry::FLAG_USER;

            if header.p_flags & program_header::PF_X != 0 {
                flags |= PageTableEntry::FLAG_EXECUTE;
            }

            if header.p_flags & program_header::PF_W != 0 {
                flags |= PageTableEntry::FLAG_WRITE;
            }

            if header.p_flags & program_header::PF_R != 0 {
                flags |= PageTableEntry::FLAG_READ;
            }

            unsafe {
                let offset = unaligned_vm_range.start - vm_range.start;
                let dst = frame_paddr.to_hhdm().to_mut_ptr::<u8>().add(offset);
                let src = &elf_file[file_range.clone()];

                // copy
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());

                let bss_size = unaligned_vm_range.len() - file_range.len();

                if bss_size != 0 {
                    // zero out the rest
                    core::ptr::write_bytes(dst.add(src.len()), 0, bss_size);
                }

                // map program section to page table
                {
                    let mut frame_mgr = get_frame_mgr();
                    map_range(
                        &mut frame_mgr,
                        page_table,
                        VAddr::from_raw_addr(vm_range.start),
                        frame_paddr,
                        frame_count,
                        flags,
                    )
                };
            }
        }

        // Kernel stack: 8 KiB
        let kernel_stack_frame_count = 2;
        let kernel_stack_paddr =
            { get_frame_mgr().alloc(kernel_stack_frame_count) }.expect("out of memory");
        let kernel_stack = TaskMemRange::new(
            kernel_stack_paddr.to_hhdm(),
            kernel_stack_paddr.to_hhdm() + kernel_stack_frame_count * FrameMgr::FRAME_SIZE,
        );
        allocated_frames.push(TaskAllocatedFrame::new(
            kernel_stack_paddr,
            kernel_stack_frame_count,
        ));

        // User stack: 16 KiB
        let user_stack_frame_count = 4;
        let user_stack_paddr =
            { get_frame_mgr().alloc(user_stack_frame_count) }.expect("out of memory");
        let user_stack = TaskMemRange::new(
            Self::USER_TOP_0,
            Self::USER_TOP_0 - user_stack_frame_count * FrameMgr::FRAME_SIZE,
        );
        allocated_frames.push(TaskAllocatedFrame::new(
            user_stack_paddr,
            user_stack_frame_count,
        ));

        // User heap: 0 KiB
        let user_heap_top = VAddr::from_raw_addr(program_end_vaddr + 4096);
        let user_heap_bottom = user_heap_top;
        let user_heap = TaskMemRange::new(user_heap_bottom, user_heap_top);

        // map user stack
        {
            let mut frame_mgr = get_frame_mgr();
            map_range(
                &mut frame_mgr,
                page_table,
                user_stack.bottom,
                user_stack_paddr,
                user_stack_frame_count,
                PageTableEntry::FLAG_READ
                    | PageTableEntry::FLAG_WRITE
                    | PageTableEntry::FLAG_USER
                    | PageTableEntry::FLAG_ACCESSED
                    | PageTableEntry::FLAG_DIRTY,
            );
        }

        let context = TaskContext::create_user_context(
            kernel_stack.top,
            user_stack.top,
            VAddr::from_raw_addr(elf.entry as usize),
        );

        Self {
            pid,
            state: TaskState::Ready,
            context,
            page_table_config,
            kernel_stack,
            user_stack,
            user_heap,
            allocated_frames,
        }
    }

    pub fn pid(&self) -> usize {
        self.pid
    }

    pub fn state(&self) -> TaskState {
        self.state
    }

    pub fn context(&self) -> &TaskContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut TaskContext {
        &mut self.context
    }

    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    pub ra: usize,
    pub sp: usize,
    pub s: [usize; 12],
}

impl TaskContext {
    pub const EMPTY: Self = Self {
        ra: 0,
        sp: 0,
        s: [0; 12],
    };

    pub fn create_user_context(
        kernel_stack_top: VAddr,
        user_stack_top: VAddr,
        entry: VAddr,
    ) -> Self {
        Self {
            ra: task_entry_wrapper as usize,
            sp: kernel_stack_top.addr,
            s: [
                user_stack_top.addr,
                entry.addr,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
        }
    }
}

#[unsafe(naked)]
extern "C" fn task_entry_wrapper() -> ! {
    core::arch::naked_asm!("move a0, s0", "move a1, s1", "call task_entry");
}

#[unsafe(no_mangle)]
extern "C" fn task_entry(user_stack_top: usize, entry: usize) -> ! {
    let mut kernel_sp: usize;

    // back-up kernel stack pointer
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) kernel_sp);
        core::arch::asm!("csrw sscratch, {}", in(reg) kernel_sp);
    }

    // setup user mode and interrupts
    unsafe {
        let mut sstatus: usize;
        core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);

        // spp = 0 (user mode)
        // (supervisor previous privilege level)
        sstatus &= !(1 << 8);

        // spie = 1
        // (supervisor previous interrupt enable)
        sstatus |= 1 << 5;

        core::arch::asm!("csrw sstatus, {}", in(reg) sstatus);
    }

    // switch to user stack
    unsafe {
        core::arch::asm!("mv sp, {}", in(reg) user_stack_top);
    }

    // setup entry point
    unsafe {
        core::arch::asm!("csrw sepc, {}", in(reg) entry);
        core::arch::asm!("li a0, 0");
        core::arch::asm!("li a1, 0");
        core::arch::asm!("li ra, 0");
    }

    // jump to entry point
    unsafe {
        core::arch::asm!("sret", options(noreturn));
    }
}
