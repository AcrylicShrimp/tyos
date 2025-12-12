pub(crate) fn syscall(syscall_id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let mut result: isize;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") syscall_id,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            lateout("a0") result,
        );
    }

    result
}
