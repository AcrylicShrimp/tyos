pub mod address_space;
pub mod elf_importer;
pub mod loader;
pub mod program;

use crate::task::{address_space::TaskAddressSpace, loader::LoadedProgram};
use tyos_mem::{
    addr::VAddr,
    page_table_config::{PageTableConfig, PageTableMode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Stopping,
    Exited,
}

#[derive(Debug)]
pub struct Task {
    pid: usize,
    state: TaskState,
    context: TaskContext,
    addr_space: Option<TaskAddressSpace>,
}

impl Task {
    pub fn empty(pid: usize) -> Self {
        Self {
            pid,
            state: TaskState::Ready,
            context: TaskContext::EMPTY,
            addr_space: None,
        }
    }

    pub fn from_loaded_program(pid: usize, program: LoadedProgram) -> Self {
        Self {
            pid,
            state: TaskState::Ready,
            context: TaskContext::create_user_context(
                PageTableConfig::new(
                    PageTableMode::Sv39,
                    0,
                    program.addr_space.root_page_frame().ppn(),
                ),
                program.addr_space.kernel_stack_range().end,
                program.addr_space.user_main_thread_stack_range().end,
                program.entry,
            ),
            addr_space: Some(program.addr_space),
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

    pub fn addr_space(&self) -> Option<&TaskAddressSpace> {
        self.addr_space.as_ref()
    }

    pub fn addr_space_mut(&mut self) -> Option<&mut TaskAddressSpace> {
        self.addr_space.as_mut()
    }

    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }

    pub unsafe fn destroy_addr_space(mut self) {
        if let Some(addr_space) = self.addr_space.take() {
            unsafe {
                addr_space.destroy();
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    pub satp: usize,
    pub ra: usize,
    pub sp: usize,
    pub s: [usize; 12],
}

impl TaskContext {
    pub const EMPTY: Self = Self {
        satp: 0,
        ra: 0,
        sp: 0,
        s: [0; 12],
    };

    pub fn create_user_context(
        page_table_config: PageTableConfig,
        kernel_stack_top: VAddr,
        user_main_thread_stack_top: VAddr,
        entry: VAddr,
    ) -> Self {
        Self {
            satp: page_table_config.config(),
            ra: task_entry_wrapper as *const () as usize,
            sp: kernel_stack_top.addr,
            s: [
                user_main_thread_stack_top.addr,
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
