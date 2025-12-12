use crate::scheduler;
use tyos_syscall::SyscallId;
use tyos_uart::print;

pub fn syscall_handler(syscall_id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    match syscall_id {
        SyscallId::WRITE => {
            let fd = arg0;
            let buf = arg1 as *const u8;
            let len = arg2;
            sys_write(fd, buf, len)
        }
        SyscallId::EXIT => {
            let code = arg0 as isize;
            sys_exit(code)
        }
        _ => -1,
    }
}

fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        1 => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };

            match core::str::from_utf8(slice) {
                Ok(str) => {
                    print!("{}", str);
                    len as isize
                }
                Err(_) => -1,
            }
        }
        _ => -1,
    }
}

fn sys_exit(code: isize) -> ! {
    scheduler::exit_task(scheduler::current_pid(), code);
    unreachable!()
}
