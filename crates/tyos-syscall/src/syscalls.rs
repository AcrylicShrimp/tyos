use crate::{SyscallId, syscall::syscall};

pub fn exit(code: isize) -> ! {
    syscall(SyscallId::EXIT, code as usize, 0, 0);
    unreachable!()
}

pub fn grow_heap(page_count: usize) -> isize {
    syscall(SyscallId::GROW_HEAP, page_count, 0, 0)
}

pub fn read(fd: usize, buf: *mut u8, len: usize) -> isize {
    syscall(SyscallId::READ, fd, buf as usize, len)
}

pub fn write(fd: usize, buf: *const u8, len: usize) -> isize {
    syscall(SyscallId::WRITE, fd, buf as usize, len)
}
