use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
    task::Waker,
};

use crate::{runtime::add_waker, task::waker_ext::WakerSet};

pub struct AsyncMutex<T> {
    // 主要使用原子类型的内部可变性
    occupied: AtomicBool,

    // 等待中的任务
    waiting_wakers: UnsafeCell<WakerSet>,

    data: UnsafeCell<T>,
}

impl<T> AsyncMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            occupied: AtomicBool::new(false),
            waiting_wakers: UnsafeCell::new(WakerSet::default()),
            data: UnsafeCell::new(value),
        }
    }

    pub async fn lock(&self) -> AsyncMutexGuard<'_, T> {
        let mut guard = AsyncMutexGuard::new(self);
        (&mut guard).await;
        guard
    }

    unsafe fn waiting_wakers(&self) -> &mut WakerSet {
        unsafe { &mut *self.waiting_wakers.get() }
    }

    fn add_waiting_waker(&self, waker: Waker) {
        unsafe { self.waiting_wakers().add_waker(waker) };
    }

    fn pop_waker(&self) -> Option<Waker> {
        unsafe { self.waiting_wakers().pop() }
    }
}

pub struct AsyncMutexGuard<'a, T> {
    mtx: &'a AsyncMutex<T>,
}

impl<'a, T> AsyncMutexGuard<'a, T> {
    fn new(mtx: &'a AsyncMutex<T>) -> Self {
        Self { mtx }
    }
}

impl<T> Deref for AsyncMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mtx.data.get() }
    }
}

impl<T> DerefMut for AsyncMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mtx.data.get() }
    }
}

impl<T> Future for AsyncMutexGuard<'_, T> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self
            .mtx
            .occupied
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            self.mtx.add_waiting_waker(cx.waker().clone());
            std::task::Poll::Pending
        } else {
            // 抢占成功
            std::task::Poll::Ready(())
        }
    }
}

impl<T> Drop for AsyncMutexGuard<'_, T> {
    fn drop(&mut self) {
        // 释放占用
        assert_eq!(
            self.mtx
                .occupied
                .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed),
            Ok(true)
        );

        if let Some(waker) = self.mtx.pop_waker() {
            add_waker(waker);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ops::Deref, rc::Rc, time};

    use crate::{sleep, sync::mutex::AsyncMutex};

    #[rt_entry::test]
    async fn test_mtx() {
        let mtx = Rc::new(AsyncMutex::new(Vec::<usize>::new()));

        async fn inner(m: Rc<AsyncMutex<Vec<usize>>>, elem: usize) {
            let mut guard = m.lock().await;
            guard.push(elem);
            log::info!("add {}", elem);
            sleep(time::Duration::from_millis(100)).await;
        }

        for i in 0..8 {
            spawn!(inner(mtx.clone(), i));
        }

        sleep(time::Duration::from_millis(1000)).await;
        let v = mtx.lock().await;
        log::info!("final vector: {:?}", v.deref());
    }

    #[rt_entry::test]
    async fn test_dead_lock() {
        let mtx1 = Rc::new(AsyncMutex::new(1));
        let mtx2 = Rc::new(AsyncMutex::new(2));

        async fn inner(m1: Rc<AsyncMutex<usize>>, m2: Rc<AsyncMutex<usize>>) {
            let _guard1 = m1.lock().await;
            log::info!("first get {} lock", _guard1.deref());
            sleep(time::Duration::from_millis(1)).await;
            let _guard2 = m2.lock().await;
            log::info!("second get {} lock", *_guard2.deref());
        }

        spawn!(inner(mtx1.clone(), mtx2.clone()));
        spawn!(inner(mtx2.clone(), mtx1.clone()));
        // spawn!(inner(mtx1.clone(), mtx2.clone()));
    }
}
