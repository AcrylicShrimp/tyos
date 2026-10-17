use core::marker::PhantomData;
use tyos_arch::raw::csr::sstatus::{self, SstatusFlags};

unsafe fn local_irq_save() -> IrqState {
    let sstatus = unsafe { sstatus::read_and_clear_flags(SstatusFlags::SIE) };

    IrqState {
        was_enabled: sstatus.contains(SstatusFlags::SIE),
    }
}

unsafe fn local_irq_restore(state: IrqState) {
    if state.was_enabled {
        unsafe {
            sstatus::set_flags(SstatusFlags::SIE);
        }
    } else {
        unsafe {
            sstatus::clear_flags(SstatusFlags::SIE);
        }
    }
}

#[derive(Debug)]
struct IrqState {
    was_enabled: bool,
}

#[derive(Debug)]
pub struct LocalIrqGuard {
    state: Option<IrqState>,
    _phantom: PhantomData<*mut ()>,
}

impl LocalIrqGuard {
    pub fn disable() -> Self {
        Self {
            state: Some(unsafe { local_irq_save() }),
            _phantom: PhantomData,
        }
    }
}

impl Drop for LocalIrqGuard {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            unsafe {
                local_irq_restore(state);
            }
        }
    }
}
