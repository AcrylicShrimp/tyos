mod sstatus_sum_guard;

use crate::{scheduler, syscall::sstatus_sum_guard::SstatusSumGuard};
use tyos_syscall::SyscallId;
use tyos_uart::print;

pub fn syscall_handler(syscall_id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    match syscall_id {
        SyscallId::EXIT => {
            let code = arg0 as isize;
            sys_exit(code)
        }
        SyscallId::GROW_HEAP => {
            let page_count = arg0;
            sys_grow_heap(page_count)
        }
        SyscallId::READ => {
            let fd = arg0;
            let buf = arg1 as *mut u8;
            let len = arg2;
            sys_read(fd, buf, len)
        }
        SyscallId::WRITE => {
            let fd = arg0;
            let buf = arg1 as *const u8;
            let len = arg2;
            sys_write(fd, buf, len)
        }
        _ => -1,
    }
}

fn sys_exit(code: isize) -> ! {
    scheduler::exit_task(scheduler::current_pid(), code);
    unreachable!("sys_exit should not return");
}

fn sys_grow_heap(page_count: usize) -> isize {
    match scheduler::with_current_address_space(|addr_space| {
        let old_user_heap_end = addr_space.user_heap_range().end;
        addr_space.grow_heap(page_count).map(|_| old_user_heap_end)
    }) {
        Some(Ok(old_user_heap_end)) => old_user_heap_end.addr as isize,
        _ => 0,
    }
}

fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    match fd {
        0 => {
            if len == 0 {
                return 0;
            }

            let _sstatus_guard = SstatusSumGuard::new();
            let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
            let mut len = 0;

            for dst in &mut slice[..] {
                if let Some(byte) = tyos_uart::Uart.try_read_byte() {
                    *dst = byte;
                    len += 1;
                } else {
                    break;
                }
            }

            len
        }
        _ => -1,
    }
}

fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        1 => {
            let _sstatus_guard = SstatusSumGuard::new();
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
