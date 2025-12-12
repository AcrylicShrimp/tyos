use crate::{
    pid_allocator::PidAllocator,
    task::{Task, TaskContext, TaskState},
};
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use goblin::elf::Elf;
use spinning_top::Spinlock;

static SCHEDULER: Spinlock<Scheduler> = Spinlock::new(Scheduler::new());

pub fn current_pid() -> usize {
    let scheduler = SCHEDULER.lock();

    scheduler.current_pid()
}

pub fn spawn_idle_task() {
    let mut scheduler = SCHEDULER.lock();

    scheduler.spawn_idle_task();
}

pub fn spawn_task(elf_file: &[u8]) {
    let mut scheduler = SCHEDULER.lock();

    scheduler.spawn_task(elf_file);
}

pub fn exit_task(pid: usize, exit_code: isize) {
    let context_switcher = {
        let mut scheduler = SCHEDULER.lock();

        if scheduler.stop_task(pid, exit_code) {
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
}

#[derive(Debug, Clone, Copy)]
struct ExitedTask {
    pid: usize,
    exit_code: isize,
}

struct Scheduler {
    task_map: BTreeMap<usize, Task>,
    pid_allocator: PidAllocator,
    exited_tasks: Vec<ExitedTask>,
    current_pid: usize,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            task_map: BTreeMap::new(),
            pid_allocator: PidAllocator::new(),
            exited_tasks: Vec::new(),
            current_pid: 0,
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
        let elf = match Elf::parse(elf_file) {
            Ok(elf) => elf,
            Err(err) => {
                panic!("failed to parse elf file: {err:#?}");
            }
        };

        let pid = self.pid_allocator.alloc();
        let task = Task::load_from_elf(pid, &elf, elf_file);
        self.task_map.insert(pid, task);
    }

    fn stop_task(&mut self, pid: usize, exit_code: isize) -> bool {
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

        if task.state() == TaskState::Exited {
            return false;
        }

        task.set_state(TaskState::Exited);
        self.exited_tasks.push(ExitedTask { pid, exit_code });
        self.current_pid = 0;

        true
    }

    fn schedule(&mut self) -> Option<ContextSwitcher> {
        self.cleanup_exited_tasks();

        if self.task_map.is_empty() {
            return None;
        }

        let next_pid = self
            .task_map
            .range((self.current_pid + 1)..)
            .next()
            .map(|(pid, _)| *pid)
            .unwrap_or_else(|| *self.task_map.keys().next().unwrap_or(&0));

        if next_pid == self.current_pid {
            return None;
        }

        let current_task_context = match self.task_map.get_mut(&self.current_pid) {
            Some(task) => {
                task.set_state(TaskState::Ready);
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

    fn cleanup_exited_tasks(&mut self) {
        for exited_task in self.exited_tasks.iter() {
            if exited_task.pid == self.current_pid {
                continue;
            }

            if self.task_map.remove(&exited_task.pid).is_some() {
                // TODO: store the exit code and pass to the joiner
                // NOTE: the joiner can be implemented as a object that waits for the task and returns the exit code;
                self.pid_allocator.free(exited_task.pid);
            }
        }

        self.exited_tasks.clear();
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
        "sd ra, 0 * 8(a0)",
        "sd sp, 1 * 8(a0)",
        "sd s0, 2 * 8(a0)",
        "sd s1, 3 * 8(a0)",
        "sd s2, 4 * 8(a0)",
        "sd s3, 5 * 8(a0)",
        "sd s4, 6 * 8(a0)",
        "sd s5, 7 * 8(a0)",
        "sd s6, 8 * 8(a0)",
        "sd s7, 9 * 8(a0)",
        "sd s8, 10 * 8(a0)",
        "sd s9, 11 * 8(a0)",
        "sd s10, 12 * 8(a0)",
        "sd s11, 13 * 8(a0)",
        "",
        "ld ra, 0 * 8(a1)",
        "ld sp, 1 * 8(a1)",
        "ld s0, 2 * 8(a1)",
        "ld s1, 3 * 8(a1)",
        "ld s2, 4 * 8(a1)",
        "ld s3, 5 * 8(a1)",
        "ld s4, 6 * 8(a1)",
        "ld s5, 7 * 8(a1)",
        "ld s6, 8 * 8(a1)",
        "ld s7, 9 * 8(a1)",
        "ld s8, 10 * 8(a1)",
        "ld s9, 11 * 8(a1)",
        "ld s10, 12 * 8(a1)",
        "ld s11, 13 * 8(a1)",
        "",
        "ret",
    );
}
