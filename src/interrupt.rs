pub fn enable_interrupts() {
    /// Software Interrupt Enable
    const SIE_SSIE: usize = 1 << 1;
    /// Timer Interrupt Enable
    const SIE_STIE: usize = 1 << 5;
    /// External Interrupt Enable
    const SIE_SEIE: usize = 1 << 9;

    /// Global Interrupt Enable
    const SSTATUS_SIE: usize = 1 << 1;

    unsafe {
        core::arch::asm!("csrs sie, {}", in(reg) SIE_SSIE | SIE_STIE | SIE_SEIE);
        core::arch::asm!("csrs sstatus, {}", in(reg) SSTATUS_SIE);
    }
}
