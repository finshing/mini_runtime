use std::time;

use std::task::Waker;

use crate::{
    collections::box_ptr_set::BoxPtrSetDropper,
    io_event::{Event, IoEvent},
    result::Result,
    timer::PriorityTimerQueue,
};

pub struct Poller {
    timer_queue: PriorityTimerQueue,
    net_poll: mio::Poll,
}

impl Poller {
    pub fn new() -> Result<Self> {
        Ok(Self {
            timer_queue: PriorityTimerQueue::default(),
            net_poll: mio::Poll::new()?,
        })
    }

    pub fn poll(&mut self) -> Vec<Waker> {
        let mut wakers = Vec::new();
        let delay = self.timer_queue.delay();

        wakers.extend(self.io_poll(delay));
        wakers.extend(self.timer_queue.get_wakers());

        wakers
    }

    fn io_poll(&mut self, timeout: Option<time::Duration>) -> Vec<Waker> {
        log::trace!("net_poll timeout: {:?}", timeout);

        let mut wakers = Vec::new();
        let mut events = mio::event::Events::with_capacity(1024);
        match self.net_poll.poll(&mut events, timeout) {
            Ok(()) => {
                for e in events.iter() {
                    let io_event = unsafe { IoEvent::from_token(e.token()) };
                    wakers.extend(io_event.read_events(e));
                }
            }
            Err(e) => {
                log::warn!("net_poll.poll failed: {}", e);
            }
        }

        wakers
    }

    pub fn add_timer(&mut self, wake_at: time::Instant, waker: Waker) -> BoxPtrSetDropper<Waker> {
        self.timer_queue.add_timer(wake_at, waker)
    }

    pub fn register<S: mio::event::Source>(
        &mut self,
        events: Vec<crate::io_event::Event>,
        // 这里传入IoEvent而非直接传Token的原因在于保证反解析时候的类型一致性
        io_event: &IoEvent,
        source: &mut S,
    ) -> Result<()> {
        if let Some(interests) = Event::to_interests(events) {
            self.net_poll
                .registry()
                .register(source, io_event.token(), interests)?;
        }

        Ok(())
    }

    pub fn reregister<S: mio::event::Source>(
        &mut self,
        events: Vec<crate::io_event::Event>,
        io_event: &IoEvent,
        source: &mut S,
    ) -> Result<()> {
        if let Some(interests) = Event::to_interests(events) {
            self.net_poll
                .registry()
                .reregister(source, io_event.token(), interests)?;
        }

        Ok(())
    }

    pub fn deregister<S: mio::event::Source>(&mut self, source: &mut S) -> Result<()> {
        Ok(self.net_poll.registry().deregister(source)?)
    }
}
