use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug)]
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock<'l>(&'l self) -> SpinLockGuard<'l, T> {
        self.acquire_lock();
        SpinLockGuard {
            lock: self,
            _phantom: PhantomData,
        }
    }

    fn acquire_lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    unsafe fn unlock_unchecked(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

unsafe impl<T: Send> Sync for SpinLock<T> {}

unsafe impl<T: Send> Send for SpinLock<T> {}

pub struct SpinLockGuard<'l, T> {
    lock: &'l SpinLock<T>,
    _phantom: PhantomData<&'l mut T>,
}

impl<T> SpinLockGuard<'_, T> {
    pub fn get(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.unlock_unchecked();
        }
    }
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.get()
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}
