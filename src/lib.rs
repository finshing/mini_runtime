#![allow(clippy::non_canonical_partial_ord_impl)]
#![allow(clippy::mut_from_ref)]

use core::time;

use crate::{
    runtime::{can_finish, get_waker, wait},
    timer::Sleeper,
};

pub(crate) mod helper;
pub(crate) mod runtime;
pub(crate) mod task;
pub(crate) mod timer;

pub use runtime::spawn;

pub fn sleep(delay: time::Duration) -> Sleeper {
    Sleeper::delay(delay)
}

// 运行直到无运行中的任务时候
pub fn run() {
    loop {
        while let Some(waker) = get_waker() {
            waker.wake();
        }

        // 判断是否还有多余的任务
        if can_finish() {
            break;
        }

        wait();
    }

    println!("runtime done");
}
