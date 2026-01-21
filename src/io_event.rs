use std::{cell::RefCell, collections::HashSet, io, sync::Once, task::Waker};

use mio::Token;

use crate::{
    result::Result,
    task::{TaskAttr, task_id::TaskId, waker_ext::WakerExt},
};

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Event {
    Read,
    Write,
}

impl Event {
    pub fn to_interest(&self) -> mio::Interest {
        match self {
            Event::Read => mio::Interest::READABLE,
            Event::Write => mio::Interest::WRITABLE,
        }
    }

    // mio::Interest的各个值之间可以进行或运算
    pub fn to_interests(events: Vec<Event>) -> Option<mio::Interest> {
        let interests: Option<mio::Interest> = None;
        for event in events {
            interests.map_or_else(|| event.to_interest(), |is| is.add(event.to_interest()));
        }
        interests
    }
}

#[derive(Default)]
pub struct IoEvent {
    // 等待读事件的Waker
    read_wakers: HashSet<WakerExt>,

    // 等待写事件的Waker
    write_wakers: HashSet<WakerExt>,
}

impl IoEvent {
    // 由于我们要使用Token和IoEvent互相映射，所以需要保证IoEvent的地址不发生变化
    pub fn new() -> Box<Self> {
        Box::new(Self::default())
    }

    pub fn token(&self) -> Token {
        Token(self as *const _ as usize)
    }

    // 添加等待事件的Waker
    pub fn wait(&mut self, event: Event, waker_ext: WakerExt) {
        match event {
            Event::Read => self.read_wakers.insert(waker_ext),
            Event::Write => self.write_wakers.insert(waker_ext),
        };
    }

    // 事件就绪，并获取就绪的全部Waker
    pub fn event_ready(&mut self, event: &mio::event::Event) -> Vec<Waker> {
        let mut wakers: Vec<Waker> = Vec::new();
        if event.is_readable() {
            for waker_ext in self.read_wakers.drain() {
                wakers.push(waker_ext.0);
            }
        }
        if event.is_writable() {
            for waker_ext in self.write_wakers.drain() {
                wakers.push(waker_ext.0);
            }
        }

        wakers
    }

    fn is_event_ready(&self, event: Event, waker: &Waker) -> bool {
        let task_attr = unsafe { TaskAttr::from_raw_data(waker.data()) };
        match event {
            Event::Read => self.read_wakers.contains(&task_attr.tid),
            Event::Write => self.write_wakers.contains(&task_attr.tid),
        }
    }
}

pub trait TReregister<'a> {
    // 支持不同io类型的事件重新注册，方便进行读或者写的切换。
    fn reregister(&'a mut self, events: Vec<Event>) -> Result<&'a mut IoEvent>;
}

// io事件处理Future
pub struct IoEventHandler<'a> {
    event: Event,
    // 这里使用RefCell主要是方便在self.once时使用
    io_event: RefCell<&'a mut IoEvent>,
    // 部分场景下，IoEventHandler可能都没有执行就被删除了（如之后要实现的select里），这时候也希望从相应的队列里也删除掉对应的Waker，避免被意外的唤醒
    tid: RefCell<Option<TaskId>>,
    once: Once,
}

impl<'a> IoEventHandler<'a> {
    fn new(event: Event, io_event: &'a mut IoEvent) -> Self {
        Self {
            event,
            io_event: RefCell::new(io_event),
            tid: RefCell::new(None),
            once: Once::new(),
        }
    }
}

impl<'a> Future for IoEventHandler<'a> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.once.call_once(|| {
            let waker_ext: WakerExt = cx.waker().clone().into();
            self.tid
                .borrow_mut()
                .replace(waker_ext.get_task_attr().tid.clone());
            self.io_event.borrow_mut().wait(self.event, waker_ext);
        });

        if self
            .io_event
            .borrow_mut()
            .is_event_ready(self.event, cx.waker())
        {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    }
}

impl<'a> Drop for IoEventHandler<'a> {
    fn drop(&mut self) {
        if let Some(tid) = self.tid.borrow().as_ref() {
            self.io_event.borrow_mut().read_wakers.remove(tid);
        }
    }
}
