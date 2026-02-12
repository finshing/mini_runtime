use std::{cell::RefCell, sync::Once};

use crate::{
    runtime::add_waker,
    task::{
        TaskAttr,
        waker_ext::{WakerSet, WakerSetDropper},
    },
};

#[derive(Default)]
pub struct Notifier {
    waiting_wakers: WakerSet,
}

impl Notifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn wait(&self) {
        NotifierWaiter::new(self).await;
    }

    pub fn notify_one(&self) {
        if let Some(waker_ext) = self.waiting_wakers.pop() {
            add_waker(waker_ext.into());
        }
    }

    pub fn notify_all(&self) {
        for waker_ext in self.waiting_wakers.drain() {
            add_waker(waker_ext.into());
        }
    }
}

pub struct NotifierWaiter<'a> {
    notifier: &'a Notifier,
    once: Once,
    _dropper: RefCell<Option<WakerSetDropper>>,
}

impl<'a> NotifierWaiter<'a> {
    fn new(notifier: &'a Notifier) -> Self {
        Self {
            notifier,
            once: Once::new(),
            _dropper: RefCell::new(None),
        }
    }
}

impl<'a> Future for NotifierWaiter<'a> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.once.call_once(|| {
            let dropper = self
                .notifier
                .waiting_wakers
                .add_with_dropper(cx.waker().clone().into());
            self._dropper.borrow_mut().replace(dropper);
        });

        let tid = unsafe { TaskAttr::from_raw_data(cx.waker().data()) }.get_tid();
        // notifier在进行通知的时候会移除相应的唤醒器
        if self.notifier.waiting_wakers.contains(tid) {
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, time};

    use crate::{sleep, sync::notifier::Notifier};

    #[rt_entry::test]
    async fn test_notifier() {
        let notifier = Rc::new(Notifier::new());

        async fn inner(notifier: Rc<Notifier>, num: usize) {
            notifier.wait().await;
            log::info!("waiter to run - {}", num);
        }

        for i in 0..10 {
            spawn!(inner(notifier.clone(), i));
        }

        for _ in 0..5 {
            sleep(time::Duration::from_millis(200)).await;
            notifier.notify_one();
        }

        sleep(time::Duration::from_millis(300)).await;
        notifier.notify_all();
    }

    #[rt_entry::test]
    async fn test_aba() {
        let na = Rc::new(Notifier::new());
        let na_c = na.clone();
        let nb = Rc::new(Notifier::new());
        let nb_c = nb.clone();

        spawn!(async move {
            for _ in 0..10 {
                na.wait().await;
                log::info!("B");
                nb.notify_one();
            }
        });

        spawn!(async move {
            for _ in 0..10 {
                na_c.notify_one();
                log::info!("A");
                nb_c.wait().await;
            }
        });
    }
}
