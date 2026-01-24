#![allow(clippy::non_canonical_partial_ord_impl)]
#![allow(clippy::mut_from_ref)]

use crate::{
    runtime::{can_finish, get_waker, wait},
    timer::Sleeper,
};
use std::io::Write;
use std::time;

pub mod config;
pub(crate) mod helper;
pub(crate) mod io_event;
pub mod macros;
pub(crate) mod poller;
pub mod result;
pub mod runtime;
pub(crate) mod task;
pub mod tcp;
pub(crate) mod timer;

use chrono::Local;
use log::{Level, LevelFilter};

fn get_level_color(level: Level) -> usize {
    match level {
        Level::Error => 31,
        Level::Warn => 33,
        Level::Info => 34,
        Level::Debug => 32,
        Level::Trace => 90,
    }
}

pub fn init_logger(level: LevelFilter) {
    env_logger::builder()
        .filter_level(level)
        .format(|buf, record| {
            writeln!(
                // 这里别忘记引入std::io::Write
                buf,
                "\u{1B}[{}m[{} [{}] - {}:{} - {}\u{1B}[0m",
                get_level_color(record.level()),
                Local::now().format("%Y-%m-%dT%H:%M:%S.%3f"),
                record.level(),
                record.file().unwrap_or_default(),
                record.line().unwrap_or_default(),
                record.args()
            )
        })
        .init();
}

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

    log::debug!("runtime done");
}
