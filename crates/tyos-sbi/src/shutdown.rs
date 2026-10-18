use crate::sbi_call::sbi_call;

const SBI_SHUTDOWN: usize = 0x53525354;

pub fn shutdown() -> ! {
    unsafe {
        sbi_call(SBI_SHUTDOWN, 0, 0, 0, 0).unwrap();
    }

    unreachable!("shutdown should not return");
}
