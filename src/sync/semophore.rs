use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
    task::Waker,
};

use crate::{runtime::add_waker, task::waker_ext::WakerSet};

pub struct AsyncSemophore {
    capacity: usize,

    count: AtomicUsize,

    waiting_wakers: UnsafeCell<WakerSet>,
}

impl AsyncSemophore {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            count: AtomicUsize::new(0),
            waiting_wakers: UnsafeCell::new(WakerSet::default()),
        }
    }

    pub async fn aquire(&self) -> AsyncSemophoreGuard<'_> {
        let mut guard = AsyncSemophoreGuard::new(self);
        (&mut guard).await;
        guard
    }

    fn add_waiting_waker(&self, waker: Waker) {
        unsafe {
            (&mut *self.waiting_wakers.get()).add_waker(waker);
        }
    }

    fn pop_waker(&self) -> Option<Waker> {
        unsafe { (&mut *self.waiting_wakers.get()).pop() }
    }
}

pub struct AsyncSemophoreGuard<'a> {
    semophore: &'a AsyncSemophore,
}

impl<'a> AsyncSemophoreGuard<'a> {
    fn new(semophore: &'a AsyncSemophore) -> Self {
        Self { semophore }
    }
}

impl<'a> Future for AsyncSemophoreGuard<'a> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.semophore.count.load(Ordering::Relaxed) == self.semophore.capacity {
            self.semophore.add_waiting_waker(cx.waker().clone());
            std::task::Poll::Pending
        } else {
            self.semophore.count.fetch_add(1, Ordering::Relaxed);
            std::task::Poll::Ready(())
        }
    }
}

impl<'a> Drop for AsyncSemophoreGuard<'a> {
    fn drop(&mut self) {
        self.semophore.count.fetch_sub(1, Ordering::Relaxed);
        if let Some(waker) = self.semophore.pop_waker() {
            add_waker(waker);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, time};

    use crate::{sleep, sync::semophore::AsyncSemophore};

    #[rt_entry::test]
    async fn test_semophore() {
        let semophore = Rc::new(AsyncSemophore::new(3));

        async fn inner(semophore: Rc<AsyncSemophore>, num: usize) {
            let _guard = semophore.aquire().await;
            log::info!("get num {}", num);
            sleep(time::Duration::from_millis(200)).await;
        }

        for i in 0..10 {
            spawn!(inner(semophore.clone(), i));
        }
    }
}
