use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
    task::Waker,
};

use crate::runtime::add_waker;

#[derive(Default)]
pub struct WaitGroup {
    count: AtomicUsize,

    // 只会等待一个任务
    waiting_waker: UnsafeCell<Option<Waker>>,
}

impl WaitGroup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&self) -> WaitGroupGuard<'_> {
        self.count.fetch_add(1, Ordering::Relaxed);
        WaitGroupGuard::new(self)
    }

    pub fn wait(&self) -> WaitingWg<'_> {
        WaitingWg::new(self)
    }

    fn sub_count(&self) -> usize {
        let cur = self.count.load(Ordering::Relaxed);
        if cur > 0 {
            // fetch_sub返回的是原始值
            self.count.fetch_sub(1, Ordering::Relaxed) - 1
        } else {
            0
        }
    }

    fn take_waker(&self) -> Option<Waker> {
        unsafe { (&mut *self.waiting_waker.get()).take() }
    }

    fn replace_waker(&self, waker: Waker) {
        unsafe { (&mut *self.waiting_waker.get()).replace(waker) };
    }
}

pub struct WaitGroupGuard<'a> {
    wg: &'a WaitGroup,
}

impl<'a> WaitGroupGuard<'a> {
    fn new(wg: &'a WaitGroup) -> Self {
        Self { wg }
    }
}

impl<'a> Drop for WaitGroupGuard<'a> {
    fn drop(&mut self) {
        if self.wg.sub_count() == 0
            && let Some(waker) = self.wg.take_waker()
        {
            add_waker(waker);
        }
    }
}

pub struct WaitingWg<'a> {
    wg: &'a WaitGroup,
}

impl<'a> WaitingWg<'a> {
    fn new(wg: &'a WaitGroup) -> Self {
        WaitingWg { wg }
    }
}

impl<'a> Future for WaitingWg<'a> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.wg.count.load(Ordering::Relaxed) == 0 {
            std::task::Poll::Ready(())
        } else {
            self.wg.replace_waker(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time;

    use crate::{
        sleep,
        sync::wait_group::{WaitGroup, WaitGroupGuard},
    };

    #[rt_entry::test]
    async fn test_wait_group() {
        log::info!("main task start");
        let wg = WaitGroup::new();

        async fn inner(_wg: WaitGroupGuard<'_>) {
            log::info!("sub task");
            sleep(time::Duration::from_millis(200)).await;
        }

        for _ in 0..5 {
            spawn!(inner(wg.add()));
        }

        wg.wait().await;
        log::info!("main task done");
    }
}
