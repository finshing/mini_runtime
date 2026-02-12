use std::{
    collections::{HashSet, VecDeque},
    task::Waker,
    time,
};

use lazy_static::lazy_static;

use crate::{
    helper::UPSafeCell,
    io_event::IoEvent,
    poller::Poller,
    result::Result,
    task::{TTaskClear, Task, task_id::TaskId},
    timer::Timer,
};

pub struct Runtime {
    // 尚在运行中的任务
    total_tasks: HashSet<TaskId>,

    // 等待被唤醒的Waker
    ready_wakers: VecDeque<Waker>,

    poller: Poller,
}

impl Runtime {
    fn new() -> Result<Self> {
        Ok(Self {
            total_tasks: HashSet::new(),
            ready_wakers: VecDeque::new(),
            poller: Poller::new()?,
        })
    }
}

struct RuntimeTaskClear {
    tid: TaskId,
}

impl TTaskClear for RuntimeTaskClear {
    fn clear(&self) {
        RUNTIME.exclusive_access().total_tasks.remove(&self.tid);
    }
}

lazy_static! {
    pub static ref RUNTIME: UPSafeCell<Runtime> =
        UPSafeCell::new(Runtime::new().expect("init Runtime failed"));
}

// 提交一个任务。类似于golang语言中的go语法
pub fn spawn<F: Future>(f: F) {
    let mut rt = RUNTIME.exclusive_access();

    let waker = Task::new_waker(f, |tid| {
        rt.total_tasks.insert(tid.clone());
        RuntimeTaskClear { tid }
    });

    rt.ready_wakers.push_back(waker);
}

// 是否还存在运行中的任务。用于runtime的退出时判断
pub(crate) fn can_finish() -> bool {
    RUNTIME.exclusive_access().total_tasks.is_empty()
}

pub(crate) fn get_waker() -> Option<Waker> {
    RUNTIME.exclusive_access().ready_wakers.pop_front()
}

// 新增任务
pub(crate) fn add_waker(waker: Waker) {
    RUNTIME.exclusive_access().ready_wakers.push_back(waker);
}

// 等待可执行任务（事件就绪）
pub(crate) fn wait() {
    let mut rt = RUNTIME.exclusive_access();
    for waker in rt.poller.poll() {
        rt.ready_wakers.push_back(waker);
    }
}

// 添加一个定时任务
pub(crate) fn add_timer(wake_at: time::Instant, waker: Waker) {
    RUNTIME
        .exclusive_access()
        .poller
        .add_timer(Timer::new(wake_at, waker));
}

pub(crate) fn register<S: mio::event::Source>(
    events: Vec<crate::io_event::Event>,
    io_event: &IoEvent,
    source: &mut S,
) -> Result<()> {
    RUNTIME
        .exclusive_access()
        .poller
        .register(events, io_event, source)
}

pub(crate) fn reregister<S: mio::event::Source>(
    events: Vec<crate::io_event::Event>,
    io_event: &IoEvent,
    source: &mut S,
) -> Result<()> {
    RUNTIME
        .exclusive_access()
        .poller
        .reregister(events, io_event, source)
}

pub(crate) fn deregister<S: mio::event::Source>(source: &mut S) -> Result<()> {
    RUNTIME.exclusive_access().poller.deregister(source)
}
