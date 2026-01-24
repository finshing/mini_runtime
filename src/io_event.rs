use std::{cell::RefCell, collections::HashSet, io, sync::Once, task::Waker};

use mio::Token;

use crate::{
    result::Result,
    runtime::reregister,
    task::{
        TaskAttr,
        task_id::TaskId,
        waker_ext::{WakerExt, WakerSet},
    },
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
        let mut interests: Option<mio::Interest> = None;
        for event in events {
            interests = Some(
                interests.map_or_else(|| event.to_interest(), |is| is.add(event.to_interest())),
            );
        }
        interests
    }
}

#[derive(Default)]
pub struct IoEvent {
    // 等待读事件的Waker
    read_wakers: WakerSet,

    // 等待写事件的Waker
    write_wakers: WakerSet,
}

impl IoEvent {
    // 由于我们要使用Token和IoEvent互相映射，所以需要保证IoEvent的地址不发生变化
    pub fn new() -> Box<Self> {
        Box::new(Self::default())
    }

    pub fn token(&self) -> Token {
        Token(self as *const _ as usize)
    }

    pub unsafe fn from_token(token: Token) -> &'static mut Self {
        unsafe { &mut *(token.0 as *const Self as *mut _) }
    }

    pub fn reregister<S: mio::event::Source>(
        &mut self,
        source: &mut S,
        event: Event,
    ) -> Result<IoEventHandler<'_>> {
        IoEventHandler::new(event, self, source)
    }

    // 添加等待事件的Waker
    pub fn wait(&mut self, event: Event, waker: Waker) {
        match event {
            Event::Read => self.read_wakers.add_waker(waker),
            Event::Write => self.write_wakers.add_waker(waker),
        };
    }

    // 事件就绪，并获取就绪的全部Waker
    pub fn read_events(&mut self, event: &mio::event::Event) -> Vec<Waker> {
        let mut wakers: Vec<Waker> = Vec::new();
        if event.is_readable() {
            wakers.extend(self.read_wakers.drain());
        }
        if event.is_writable() {
            wakers.extend(self.write_wakers.drain());
        }

        wakers
    }

    fn is_event_ready(&self, event: Event, waker: &Waker) -> bool {
        let task_attr = unsafe { TaskAttr::from_raw_data(waker.data()) };
        // 就绪的waker都已在poll的时候被取出（注意这里使用!取反）
        !match event {
            Event::Read => self.read_wakers.contains(&task_attr.tid),
            Event::Write => self.write_wakers.contains(&task_attr.tid),
        }
    }
}

// io事件处理Future
pub struct IoEventHandler<'a> {
    event: Event,
    // 这里使用RefCell主要是方便在self.once时使用
    io_event: RefCell<&'a mut IoEvent>,
    once: Once,
}

impl<'a> IoEventHandler<'a> {
    fn new<S: mio::event::Source>(
        event: Event,
        io_event: &'a mut IoEvent,
        source: &mut S,
    ) -> Result<Self> {
        reregister(vec![event], io_event, source)?;
        Ok(Self {
            event,
            io_event: RefCell::new(io_event),
            once: Once::new(),
        })
    }
}

impl<'a> Future for IoEventHandler<'a> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.once.call_once(|| {
            self.io_event
                .borrow_mut()
                .wait(self.event, cx.waker().clone());
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
