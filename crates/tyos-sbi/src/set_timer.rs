use crate::sbi_call::sbi_call;

const SBI_SET_TIMER: usize = 0x54494D45;

pub fn set_timer(ticks: usize) {
    unsafe {
        if let Err(err) = sbi_call(SBI_SET_TIMER, 0, ticks, 0, 0) {
            panic!("failed to set timer: {err}");
        }
    }
}

pub fn get_current_ticks() -> usize {
    let time: usize;

    unsafe {
        core::arch::asm!("csrr {}, time", out(reg) time);
    }

    time
}
