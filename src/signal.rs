use std::{
    cell::RefCell,
    sync::{
        Once,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time,
};

use lazy_static::lazy_static;
use signal_hook::{consts, low_level::register};

use crate::{
    helper::{UPSafeCell, yield_here},
    runtime::{add_waker, force_stop, spawn},
    sleep,
    task::waker_ext::{WakerSet, WakerSetDropper},
};

pub static STOPPED: AtomicBool = AtomicBool::new(false);

// 停止后最大等待时长（ms）。默认1000ms
static MAX_WAIT_DURATION_MS: AtomicU64 = AtomicU64::new(1000);

/*
 * 当前等待的在stop时执行的waker。在stop时会被移动到ready_tasks列表中并被执行
 * 由于StopWaker通常是和select!同时使用的，因此需要在删除时也同时移除这里的waker，避免二次触发waker
 */
lazy_static! {
    static ref STOP_WAITERS: UPSafeCell<WakerSet> = UPSafeCell::new(WakerSet::default());
}

// lazy_static! {
//     static ref STOP_NOTIFIER: UPSafeCell<Notifier> = UPSafeCell::new(Notifier::new());
// }

// #[allow(unused)]
// pub(crate) fn wait_stop_notifier() -> BoxedFuture<'static, ()> {
//     let mut waiter = STOP_NOTIFIER.exclusive_access().waiter();

//     Box::pin(async move {
//         waiter.wait().await;
//         Ok(())
//     })
// }

#[allow(unused)]
pub(crate) fn set_max_wait_duration(dur: time::Duration) {
    MAX_WAIT_DURATION_MS.store(dur.as_millis() as u64, Ordering::Relaxed);
}

fn get_max_wait_duration() -> time::Duration {
    time::Duration::from_millis(MAX_WAIT_DURATION_MS.load(Ordering::Relaxed))
}

fn stop_action() {
    log::warn!("signal handler: catch sigint");
    STOPPED.store(true, Ordering::Relaxed);
    // 将需要在终止时运行的任务放入待执行的任务队列中
    for waker_ext in STOP_WAITERS.exclusive_access().drain() {
        add_waker(waker_ext.into());
    }
    log::info!("stop wakers to run");

    spawn(async {
        // 等待一定时长后强制停止，但又希望在只剩当前一个waker的时候也及时停止
        // 所以需要配合Runtime::finish()一起使用
        sleep(get_max_wait_duration()).await;
        // STOP_NOTIFIER.exclusive_access().notify_all();
        // 保证全部被通知任务的执行
        yield_here().await;
        log::warn!("force stop");
        force_stop();
    });
}

pub fn signal_handler() {
    unsafe {
        register(consts::SIGINT, stop_action).unwrap();

        register(consts::SIGTERM, stop_action).unwrap();
    }
}

pub fn stopped() -> bool {
    STOPPED.load(Ordering::Acquire)
}

// 设置停止事件的唤醒器，在ctrl-c的时候会触发
#[derive(Default)]
pub struct StopWaker {}

impl StopWaker {
    pub fn wait(&self) -> StopWakerGuard {
        StopWakerGuard {
            once: Once::new(),
            _dropper: RefCell::new(None),
        }
    }
}

pub struct StopWakerGuard {
    once: Once,
    _dropper: RefCell<Option<WakerSetDropper>>,
}

impl Future for StopWakerGuard {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.once.call_once(|| {
            self._dropper.borrow_mut().replace(
                STOP_WAITERS
                    .exclusive_access()
                    .add_with_dropper(cx.waker().clone().into()),
            );
        });
        if stopped() {
            add_waker(cx.waker().clone());
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    }
}
