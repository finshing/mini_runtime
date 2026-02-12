use std::{
    cell::RefCell,
    sync::{
        Once,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::{
    runtime::add_waker,
    task::waker_ext::{WakerSet, WakerSetDropper},
};

pub struct AsyncSemophore {
    capacity: usize,

    count: AtomicUsize,

    waiting_wakers: WakerSet,
}

impl AsyncSemophore {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            count: AtomicUsize::new(0),
            waiting_wakers: WakerSet::default(),
        }
    }

    pub async fn aquire(&self) -> AsyncSemophoreGuard<'_> {
        let mut guard = AsyncSemophoreGuard::new(self);
        (&mut guard).await;
        guard
    }
}

pub struct AsyncSemophoreGuard<'a> {
    semophore: &'a AsyncSemophore,
    once: Once,
    // 是否获取到信号量：在没有获取到信号量的时候被删除不需要进行信号量的释放
    fetched: bool,
    _dropper: RefCell<Option<WakerSetDropper>>,
}

impl<'a> AsyncSemophoreGuard<'a> {
    fn new(semophore: &'a AsyncSemophore) -> Self {
        Self {
            semophore,
            once: Once::new(),
            fetched: false,
            _dropper: RefCell::new(None),
        }
    }
}

impl<'a> Future for AsyncSemophoreGuard<'a> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // 在信号量已满的情况下，加入等待队列中
        if self.semophore.count.load(Ordering::Relaxed) == self.semophore.capacity {
            self.once.call_once(|| {
                let dropper = self
                    .semophore
                    .waiting_wakers
                    .add_with_dropper(cx.waker().clone().into());
                self._dropper.borrow_mut().replace(dropper);
            });
            std::task::Poll::Pending
        } else {
            self.semophore.count.fetch_add(1, Ordering::Relaxed);
            self.get_mut().fetched = true;
            std::task::Poll::Ready(())
        }
    }
}

impl<'a> Drop for AsyncSemophoreGuard<'a> {
    fn drop(&mut self) {
        // 在获取到信号量的时候需要进行释放
        if self.fetched {
            self.semophore.count.fetch_sub(1, Ordering::Relaxed);
            if let Some(waker_ext) = self.semophore.waiting_wakers.pop() {
                add_waker(waker_ext.into());
            }
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
            sleep(time::Duration::from_millis(500)).await;
        }

        for i in 0..10 {
            spawn!(inner(semophore.clone(), i));
        }
    }
}
