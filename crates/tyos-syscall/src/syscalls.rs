use crate::{SyscallId, syscall::syscall};

/// Shuts down the system. This function does not return.
pub fn shutdown() -> ! {
    syscall(SyscallId::SHUTDOWN, 0, 0, 0);
    unreachable!()
}

/// Terminates the current process with the specified exit code.
///
/// ## Parameters
///
/// - `code`: The exit code for the process.
///
/// ## Returns
///
/// This function does not return; it terminates the current process.
pub fn exit(code: isize) -> ! {
    syscall(SyscallId::EXIT, code as usize, 0, 0);
    unreachable!()
}

/// Grows the heap by the specified number of pages.
///
/// ## Parameters
///
/// - `page_count`: The number of pages to grow the heap by.
///
/// ## Returns
///
/// The new size of the heap, or 0 on failure.
pub fn grow_heap(page_count: usize) -> isize {
    syscall(SyscallId::GROW_HEAP, page_count, 0, 0)
}

/// Reads data from the specified file descriptor.
///
/// ## Parameters
///
/// - `fd`: The file descriptor to read from.
/// - `buf`: A pointer to the buffer where the read data will be stored.
/// - `len`: The maximum number of bytes to read into the buffer.
///
/// ## Returns
///
/// The number of bytes read, or -1 on failure.
pub fn read(fd: usize, buf: *mut u8, len: usize) -> isize {
    syscall(SyscallId::READ, fd, buf as usize, len)
}

/// Writes data to the specified file descriptor.
///
/// ## Parameters
///
/// - `fd`: The file descriptor to write to.
/// - `buf`: A pointer to the buffer containing the data to write.
/// - `len`: The number of bytes to write from the buffer.
///
/// ## Returns
///
/// The number of bytes written, or -1 on failure.
pub fn write(fd: usize, buf: *const u8, len: usize) -> isize {
    syscall(SyscallId::WRITE, fd, buf as usize, len)
}

/// Spawns a new task. The actual task content is defined by the `name` and `len` parameters, and `arg` is passed to the new task as an argument.
///
/// ## Parameters
///
/// - `name`: A pointer to the name of the new task.
/// - `len`: The length of the name.
/// - `arg`: An argument to be passed to the new task.
///
/// ## Returns
///
/// The PID of the newly spawned task, or -1 on failure.
pub fn spawn(name: *const u8, len: usize, arg: usize) -> isize {
    syscall(SyscallId::SPAWN, name as usize, len, arg)
}

/// Waits for the specified task to finish.
///
/// ## Parameters
///
/// - `pid`: The PID of the task to wait for.
///
/// ## Returns
///
/// The exit code of the finished task, or -1 on timeout or failure.
pub fn wait(pid: usize) -> isize {
    syscall(SyscallId::WAIT, pid, 0, 0)
}
