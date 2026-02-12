use std::{
    cell::RefCell,
    collections::BinaryHeap,
    sync::Once,
    task::Waker,
    time::{self, Instant},
};

use crate::{
    collections::box_ptr_set::{BoxPtrSet, BoxPtrSetDropper, SetPtr},
    runtime::{add_timer, can_finish},
};

#[derive(Default)]
pub(crate) struct PriorityTimerQueue {
    inner: BinaryHeap<Timer>,
    set: BoxPtrSet<Waker>,
}

impl PriorityTimerQueue {
    pub fn add_timer(&mut self, wake_at: time::Instant, waker: Waker) -> BoxPtrSetDropper<Waker> {
        let set_ptr = self.set.insert(waker);
        self.inner.push(Timer::new(wake_at, set_ptr));

        self.set.build_dropper(set_ptr)
    }

    pub fn get_wakers(&mut self) -> Vec<Waker> {
        let now = Instant::now();
        let mut wakers = Vec::new();
        while let Some(timer) = self.inner.peek()
            && timer.wake_at <= now
        {
            let timer = self.inner.pop().unwrap();
            if let Some(waker) = self.set.remove(&timer.set_ptr) {
                wakers.push(waker);
            }
        }

        wakers
    }

    pub fn delay(&mut self) -> Option<time::Duration> {
        while let Some(timer) = self.inner.peek()
            && !self.set.contains(&timer.set_ptr)
        {
            self.inner.pop();
        }
        // 由于涉及到任务的删除操作，所以需要判断是否仍存在其它任务，避免永久的block
        if can_finish() {
            return Some(time::Duration::from_secs(0));
        }
        self.inner.peek().map(|t| t.wake_at - time::Instant::now())
    }
}

pub struct Timer {
    wake_at: time::Instant,
    set_ptr: SetPtr,
}

impl Timer {
    pub fn new(wake_at: time::Instant, set_ptr: SetPtr) -> Self {
        Self { wake_at, set_ptr }
    }
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.wake_at == other.wake_at
    }
}

impl Eq for Timer {}

impl PartialOrd for Timer {
    // BinaryHeap是最大堆，因此这里比较需要取反
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.wake_at.partial_cmp(&self.wake_at)
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.wake_at.cmp(&self.wake_at)
    }
}

pub struct Sleeper {
    wake_at: time::Instant,
    once: Once,
    _dropper: RefCell<Option<BoxPtrSetDropper<Waker>>>,
}

impl Sleeper {
    pub fn until(wake_at: time::Instant) -> Self {
        Self {
            wake_at,
            once: Once::new(),
            _dropper: RefCell::new(None),
        }
    }

    pub fn delay(delay: time::Duration) -> Self {
        Self::until(time::Instant::now() + delay)
    }
}

impl Future for Sleeper {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if time::Instant::now() < self.wake_at {
            // 保证当前timer只会添加一次
            self.once.call_once(|| {
                self._dropper
                    .borrow_mut()
                    .replace(add_timer(self.wake_at, cx.waker().clone()));
            });
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time;

    use crate::sleep;

    #[rt_entry::test]
    async fn test_sleep() {
        log::info!("in test");
        sleep(time::Duration::from_secs(1)).await;
        log::info!("test done");
    }
}
