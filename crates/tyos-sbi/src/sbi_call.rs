#[inline(always)]
pub unsafe fn sbi_call(
    eid: usize,
    fid: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
) -> Result<usize, usize> {
    let ret: usize;
    let err: usize;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            inlateout("a0") arg0 => err,
            inlateout("a1") arg1 => ret,
            in("a2") arg2,
            options(nostack)
        );
    }

    match err {
        0 => Ok(ret),
        _ => Err(err),
    }
}
