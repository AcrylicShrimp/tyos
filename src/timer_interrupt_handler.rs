use crate::{hal::get_timer_frequency, scheduler::schedule};
use tyos_sbi::set_timer::{get_current_ticks, set_timer};

pub fn compute_next_ticks() -> usize {
    const TICK_INTERVAL_MS: usize = 4;

    let delta = get_timer_frequency() * TICK_INTERVAL_MS / 1000;
    get_current_ticks() + delta
}

pub fn timer_interrupt_handler() {
    set_timer(compute_next_ticks());
    schedule();
}
