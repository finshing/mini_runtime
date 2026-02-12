use std::{
    cell::RefMut,
    collections::{HashSet, VecDeque},
    task::Waker,
    time,
};

use lazy_static::lazy_static;

use crate::{
    collections::box_ptr_set::BoxPtrSetDropper,
    helper::UPSafeCell,
    io_event::IoEvent,
    poller::Poller,
    result::Result,
    signal::stopped,
    task::{TTaskClear, Task, task_id::TaskId},
    variable_log,
};

lazy_static! {
    pub static ref RUNTIME: Runtime = Runtime::new().expect("init Runtime failed");
}

pub struct Runtime {
    // 尚在运行中的任务
    _total_tasks: UPSafeCell<HashSet<TaskId>>,

    // 等待被唤醒的Waker
    _ready_wakers: UPSafeCell<VecDeque<Waker>>,

    _poller: UPSafeCell<Poller>,
}

impl Runtime {
    fn new() -> Result<Self> {
        Ok(Self {
            _total_tasks: UPSafeCell::new(HashSet::new()),
            _ready_wakers: UPSafeCell::new(VecDeque::new()),
            _poller: UPSafeCell::new(Poller::new()?),
        })
    }

    #[inline]
    fn total_tasks(&self) -> RefMut<'_, HashSet<TaskId>> {
        self._total_tasks.exclusive_access()
    }

    fn ready_wakers(&self) -> RefMut<'_, VecDeque<Waker>> {
        self._ready_wakers.exclusive_access()
    }

    fn poller(&self) -> RefMut<'_, Poller> {
        self._poller.exclusive_access()
    }
}

struct RuntimeTaskClear {
    tid: TaskId,
}

impl TTaskClear for RuntimeTaskClear {
    fn clear(&self) {
        RUNTIME.total_tasks().remove(&self.tid);
    }
}

// 提交一个任务。类似于golang语言中的go语法
pub fn spawn<F: Future>(f: F) {
    let waker = Task::new_waker(f, |tid| {
        RUNTIME.total_tasks().insert(tid.clone());
        RuntimeTaskClear { tid }
    });

    RUNTIME.ready_wakers().push_back(waker);
}

// 是否还存在运行中的任务。用于runtime的退出时判断
pub(crate) fn can_finish() -> bool {
    let total_tasks_count =
        variable_log!(debug @ RUNTIME.total_tasks().len(), "running tasks count");
    // 通过中断停止时，还会有一个最大等待时长的协程在执行
    if stopped() {
        total_tasks_count <= 1
    } else {
        total_tasks_count == 0
    }
}

pub(crate) fn get_waker() -> Option<Waker> {
    RUNTIME.ready_wakers().pop_front()
}

// 新增任务
pub(crate) fn add_waker(waker: Waker) {
    RUNTIME.ready_wakers().push_back(waker);
}

// 等待可执行任务（事件就绪）
pub(crate) fn wait() {
    let wakers = RUNTIME.poller().poll();
    let mut ready_wakers = RUNTIME.ready_wakers();
    for waker in wakers {
        ready_wakers.push_back(waker);
    }
}

// 添加一个定时任务
pub(crate) fn add_timer(wake_at: time::Instant, waker: Waker) -> BoxPtrSetDropper<Waker> {
    RUNTIME.poller().add_timer(wake_at, waker)
}

pub(crate) fn register<S: mio::event::Source>(
    events: Vec<crate::io_event::Event>,
    io_event: &IoEvent,
    source: &mut S,
) -> Result<()> {
    RUNTIME.poller().register(events, io_event, source)
}

pub(crate) fn reregister<S: mio::event::Source>(
    events: Vec<crate::io_event::Event>,
    io_event: &IoEvent,
    source: &mut S,
) -> Result<()> {
    RUNTIME.poller().reregister(events, io_event, source)
}

pub(crate) fn deregister<S: mio::event::Source>(source: &mut S) -> Result<()> {
    RUNTIME.poller().deregister(source)
}

pub(crate) fn force_stop() {
    RUNTIME.total_tasks().clear();
}
