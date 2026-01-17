use std::{
    collections::{HashSet, VecDeque},
    task::Waker,
    thread, time,
};

use lazy_static::lazy_static;

use crate::{
    helper::UPSafeCell,
    task::{TTaskClear, Task, task_id::TaskId},
    timer::{PriorityTimerQueue, Timer},
};

lazy_static! {
    pub static ref RUNTIME: UPSafeCell<Runtime> = UPSafeCell::new(Runtime::new());
}

#[derive(Default)]
pub struct Runtime {
    // 尚在运行中的任务
    running_tasks: HashSet<TaskId>,

    // 等待被唤醒的Waker
    ready_wakers: VecDeque<Waker>,

    // 定时任务优先队列
    timer_queue: PriorityTimerQueue,
}

impl Runtime {
    fn new() -> Self {
        Self::default()
    }
}

// 是否还存在运行中的任务。用于runtime的退出时判断
pub(crate) fn can_finish() -> bool {
    RUNTIME.get_mut().running_tasks.is_empty()
}

pub(crate) fn get_waker() -> Option<Waker> {
    RUNTIME.get_mut().ready_wakers.pop_front()
}

// 等待可执行任务（事件就绪）
pub(crate) fn wait() {
    let rt = RUNTIME.get_mut();
    // 由于目前只有定时事件，所以可以保证尚有任务的时候.delay()不会返回None，因此使用unwrap()没有什么问题
    thread::sleep(rt.timer_queue.delay().unwrap());
    for waker in rt.timer_queue.get_wakers() {
        rt.ready_wakers.push_back(waker);
    }
}

struct RuntimeTaskClear {
    tid: TaskId,
}

impl TTaskClear for RuntimeTaskClear {
    fn clear(&self) {
        log::trace!("remove task {}", self.tid.as_ref());
        RUNTIME.get_mut().running_tasks.remove(&self.tid);
    }
}

// 提交一个任务。类似于golang语言中的go语法
pub fn spawn<F: Future>(f: F) {
    let rt = RUNTIME.get_mut();

    let waker = Task::new_waker(f, |tid| {
        rt.running_tasks.insert(tid.clone());
        RuntimeTaskClear { tid }
    });
    rt.ready_wakers.push_back(waker);
}

// 添加一个定时任务
pub(crate) fn add_timer(wake_at: time::Instant, waker: Waker) {
    RUNTIME
        .get_mut()
        .timer_queue
        .add_timer(Timer::new(wake_at, waker));
}
