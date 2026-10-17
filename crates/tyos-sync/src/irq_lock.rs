use crate::{
    irq::LocalIrqGuard,
    lock::{SpinLock, SpinLockGuard},
};
use core::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct IrqSpinLock<T> {
    lock: SpinLock<T>,
}

impl<T> IrqSpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            lock: SpinLock::new(value),
        }
    }

    pub fn lock<'l>(&'l self) -> IrqSpinLockGuard<'l, T> {
        IrqSpinLockGuard {
            _irq_guard: LocalIrqGuard::disable(),
            lock_guard: self.lock.lock(),
        }
    }
}

pub struct IrqSpinLockGuard<'l, T> {
    lock_guard: SpinLockGuard<'l, T>,
    _irq_guard: LocalIrqGuard,
}

impl<T> IrqSpinLockGuard<'_, T> {
    pub fn get(&self) -> &T {
        self.lock_guard.get()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.lock_guard.get_mut()
    }
}

impl<T> Deref for IrqSpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.get()
    }
}

impl<T> DerefMut for IrqSpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}
