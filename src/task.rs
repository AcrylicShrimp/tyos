use crate::{halt::halt, interrupt::enable_interrupts, scheduler::exit_task};
use alloc::{vec, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Exited,
}

pub struct Task {
    pid: usize,
    state: TaskState,
    context: TaskContext,
    stack: Vec<u8>,
}

impl Task {
    pub fn empty(pid: usize) -> Self {
        Self {
            pid,
            state: TaskState::Ready,
            context: TaskContext::EMPTY,
            stack: Vec::new(),
        }
    }

    pub fn new(pid: usize, entry: fn()) -> Self {
        let stack = vec![0; 16 * 1024];
        let stack_top = stack.as_ptr_range().end as usize;
        let context = TaskContext::create_fake_context(stack_top, pid, entry);

        Self {
            pid,
            state: TaskState::Ready,
            context,
            stack,
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

    pub fn create_fake_context(stack_top: usize, pid: usize, entry: fn()) -> Self {
        Self {
            ra: task_entry_wrapper as usize,
            sp: stack_top,
            s: [pid, entry as usize, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        }
    }
}

#[unsafe(naked)]
extern "C" fn task_entry_wrapper() -> ! {
    core::arch::naked_asm!("move a0, s0", "move a1, s1", "call task_entry");
}

#[unsafe(no_mangle)]
extern "C" fn task_entry(pid: usize, entry: extern "C" fn()) -> ! {
    enable_interrupts();
    entry();
    exit_task(pid, 0);
    halt();
}
