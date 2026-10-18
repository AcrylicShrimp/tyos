use tyos_arch::raw::csr;

pub struct SstatusSumGuard {
    old_sstatus: csr::sstatus::Sstatus,
}

impl SstatusSumGuard {
    pub fn new() -> Self {
        let old_sstatus =
            unsafe { csr::sstatus::read_and_set_flags(csr::sstatus::SstatusFlags::SUM) };

        SstatusSumGuard { old_sstatus }
    }
}

impl Drop for SstatusSumGuard {
    fn drop(&mut self) {
        if self.old_sstatus.contains(csr::sstatus::SstatusFlags::SUM) {
            unsafe { csr::sstatus::set_flags(csr::sstatus::SstatusFlags::SUM) };
        } else {
            unsafe { csr::sstatus::clear_flags(csr::sstatus::SstatusFlags::SUM) };
        }
    }
}
