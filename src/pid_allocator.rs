use alloc::vec::Vec;

pub struct PidAllocator {
    next_pid: usize,
    free_pids: Vec<usize>,
}

impl PidAllocator {
    pub const fn new() -> Self {
        Self {
            next_pid: 1,
            free_pids: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> usize {
        match self.free_pids.pop() {
            Some(pid) => pid,
            None => {
                let pid = self.next_pid;
                self.next_pid = match self.next_pid.checked_add(1) {
                    Some(pid) => pid,
                    None => {
                        panic!("pid overflow");
                    }
                };
                pid
            }
        }
    }

    pub fn free(&mut self, pid: usize) {
        self.free_pids.push(pid);
    }
}
