use crate::{SyscallId, syscall::syscall};

pub fn write(fd: usize, buf: *const u8, len: usize) -> isize {
    syscall(SyscallId::WRITE, fd, buf as usize, len)
}

pub fn exit(code: isize) -> ! {
    syscall(SyscallId::EXIT, code as usize, 0, 0);
    unreachable!()
}
