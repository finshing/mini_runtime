use std::{
    collections::BinaryHeap,
    sync::Once,
    task::Waker,
    time::{self, Instant},
};

use crate::runtime::add_timer;

#[derive(Default)]
pub(crate) struct PriorityTimerQueue {
    inner: BinaryHeap<Timer>,
}

impl PriorityTimerQueue {
    pub fn add_timer(&mut self, timer: Timer) {
        self.inner.push(timer);
    }

    pub fn get_wakers(&mut self) -> Vec<Waker> {
        let now = Instant::now();
        let mut wakers = Vec::new();
        while let Some(timer) = self.inner.peek()
            && timer.wake_at <= now
        {
            wakers.push(self.inner.pop().unwrap().waker);
        }

        wakers
    }

    pub fn delay(&self) -> Option<time::Duration> {
        self.inner.peek().map(|t| t.wake_at - time::Instant::now())
    }
}

pub struct Timer {
    wake_at: time::Instant,
    waker: Waker,
}

impl Timer {
    pub fn new(wake_at: time::Instant, waker: Waker) -> Self {
        Self { wake_at, waker }
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
}

impl Sleeper {
    pub fn until(wake_at: time::Instant) -> Self {
        Self {
            wake_at,
            once: Once::new(),
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
                add_timer(self.wake_at, cx.waker().clone());
            });
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}
