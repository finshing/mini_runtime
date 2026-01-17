use std::cell::UnsafeCell;

pub struct UPSafeCell<T> {
    inner: UnsafeCell<T>,
}

impl<T> UPSafeCell<T> {
    pub fn new(t: T) -> Self {
        Self {
            inner: UnsafeCell::new(t),
        }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

unsafe impl<T> Send for UPSafeCell<T> {}

unsafe impl<T> Sync for UPSafeCell<T> {}
