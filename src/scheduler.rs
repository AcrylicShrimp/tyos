use crate::{
    pid_allocator::PidAllocator,
    task::{
        Task, TaskContext, TaskState, address_space::TaskAddressSpace, elf_importer::import_elf,
        program::ProgramDefConfig,
    },
};
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use spinning_top::Spinlock;
use tyos_uart::println;

static SCHEDULER: Spinlock<Scheduler> = Spinlock::new(Scheduler::new());

pub fn current_pid() -> usize {
    let scheduler = SCHEDULER.lock();

    scheduler.current_pid()
}

pub fn with_current_address_space<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut TaskAddressSpace) -> R,
{
    let mut scheduler = SCHEDULER.lock();
    let current_pid = scheduler.current_pid();
    let task = scheduler.task_map.get_mut(&current_pid)?;
    let addr_space = task.addr_space_mut()?;
    Some(f(addr_space))
}

pub fn spawn_idle_task() {
    let mut scheduler = SCHEDULER.lock();

    scheduler.spawn_idle_task();
}

pub fn spawn_task(elf_file: &[u8]) {
    let mut scheduler = SCHEDULER.lock();

    scheduler.spawn_task(elf_file);
}

pub fn exit_task(pid: usize, _exit_code: isize) {
    let context_switcher = {
        let mut scheduler = SCHEDULER.lock();

        if scheduler.stop_task(pid) {
            scheduler.schedule()
        } else {
            None
        }
    };

    if let Some(context_switcher) = context_switcher {
        context_switcher.switch();
    }
}

pub fn schedule() {
    let context_switcher = {
        let mut scheduler = SCHEDULER.lock();
        scheduler.schedule()
    };

    if let Some(context_switcher) = context_switcher {
        context_switcher.switch();
    }

    // housekeeping: cleanup stopped tasks if the current task is the idle task
    {
        let mut scheduler = SCHEDULER.lock();

        if scheduler.current_pid() == 0 {
            scheduler.cleanup_stopping_tasks();
        }
    }
}

struct Scheduler {
    task_map: BTreeMap<usize, Task>,
    pid_allocator: PidAllocator,
    current_pid: usize,
    stopping_tasks: Vec<usize>,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            task_map: BTreeMap::new(),
            pid_allocator: PidAllocator::new(),
            current_pid: 0,
            stopping_tasks: Vec::new(),
        }
    }

    fn current_pid(&self) -> usize {
        self.current_pid
    }

    fn spawn_idle_task(&mut self) {
        let pid = 0;
        let task = Task::empty(pid);
        self.task_map.insert(pid, task);
    }

    fn spawn_task(&mut self, elf_file: &[u8]) {
        let program = match import_elf(
            elf_file,
            &ProgramDefConfig {
                vmem_range: TaskAddressSpace::program_vmem_range(),
            },
        ) {
            Ok(program) => program,
            Err(err) => {
                panic!("failed to import elf file: {err:#?}");
            }
        };

        println!("program is imported");

        let pid = self.pid_allocator.alloc();
        let task = Task::from_loaded_program(pid, program);

        self.task_map.insert(pid, task);
    }

    fn stop_task(&mut self, pid: usize) -> bool {
        if pid == 0 {
            // idle task cannot be stopped
            return false;
        }

        let task = match self.task_map.get_mut(&pid) {
            Some(task) => task,
            None => {
                return false;
            }
        };

        // TODO: check if the task is privileged and the caller has permission to stop it

        if task.state() == TaskState::Exited || task.state() == TaskState::Stopping {
            return false;
        }

        task.set_state(TaskState::Stopping);
        self.stopping_tasks.push(pid);

        true
    }

    fn schedule(&mut self) -> Option<ContextSwitcher> {
        if self.task_map.is_empty() {
            return None;
        }

        let next_pid = self
            .task_map
            .range((self.current_pid + 1)..)
            .find(|(_, task)| {
                task.state() != TaskState::Exited && task.state() != TaskState::Stopping
            })
            .map(|(pid, _)| *pid)
            .unwrap_or_else(|| *self.task_map.keys().next().unwrap_or(&0));

        if next_pid == self.current_pid {
            return None;
        }

        let current_task_context = match self.task_map.get_mut(&self.current_pid) {
            Some(task) => {
                if task.state() != TaskState::Exited && task.state() != TaskState::Stopping {
                    task.set_state(TaskState::Ready);
                }

                task.context_mut() as *mut _
            }
            None => {
                panic!("current task not found: {}", self.current_pid);
            }
        };
        let next_task_context = match self.task_map.get_mut(&next_pid) {
            Some(task) => {
                task.set_state(TaskState::Running);
                task.context() as *const _
            }
            None => {
                panic!("next task not found: {}", next_pid);
            }
        };

        self.current_pid = next_pid;

        Some(ContextSwitcher::new(
            current_task_context,
            next_task_context,
        ))
    }

    fn cleanup_stopping_tasks(&mut self) {
        for pid in &self.stopping_tasks {
            if *pid == self.current_pid {
                continue;
            }

            if let Some(task) = self.task_map.remove(pid) {
                // TODO: store the exit code and pass to the joiner
                // NOTE: the joiner can be implemented as a object that waits for the task and returns the exit code;
                self.pid_allocator.free(*pid);

                unsafe {
                    task.destroy_addr_space();
                }
            }
        }

        self.stopping_tasks.clear();
    }
}

pub struct ContextSwitcher {
    from: *mut TaskContext,
    to: *const TaskContext,
}

impl ContextSwitcher {
    #[must_use]
    pub fn new(from: *mut TaskContext, to: *const TaskContext) -> Self {
        Self { from, to }
    }

    pub fn switch(self) {
        unsafe {
            __switch(self.from, self.to);
        }
    }
}

#[unsafe(naked)]
unsafe extern "C" fn __switch(current: *mut TaskContext, next: *const TaskContext) {
    core::arch::naked_asm!(
        "csrr t0, satp",
        "sd t0, 0 * 8(a0)",
        "",
        "sd ra, 1 * 8(a0)",
        "sd sp, 2 * 8(a0)",
        "sd s0, 3 * 8(a0)",
        "sd s1, 4 * 8(a0)",
        "sd s2, 5 * 8(a0)",
        "sd s3, 6 * 8(a0)",
        "sd s4, 7 * 8(a0)",
        "sd s5, 8 * 8(a0)",
        "sd s6, 9 * 8(a0)",
        "sd s7, 10 * 8(a0)",
        "sd s8, 11 * 8(a0)",
        "sd s9, 12 * 8(a0)",
        "sd s10, 13 * 8(a0)",
        "sd s11, 14 * 8(a0)",
        "",
        "ld t0, 0 * 8(a1)",
        "csrw satp, t0",
        "sfence.vma",
        "",
        "ld ra, 1 * 8(a1)",
        "ld sp, 2 * 8(a1)",
        "ld s0, 3 * 8(a1)",
        "ld s1, 4 * 8(a1)",
        "ld s2, 5 * 8(a1)",
        "ld s3, 6 * 8(a1)",
        "ld s4, 7 * 8(a1)",
        "ld s5, 8 * 8(a1)",
        "ld s6, 9 * 8(a1)",
        "ld s7, 10 * 8(a1)",
        "ld s8, 11 * 8(a1)",
        "ld s9, 12 * 8(a1)",
        "ld s10, 13 * 8(a1)",
        "ld s11, 14 * 8(a1)",
        "",
        "ret",
    );
}
