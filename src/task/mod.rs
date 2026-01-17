pub mod task_id;
use std::{
    cell::RefCell,
    mem::ManuallyDrop,
    ops::Deref,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use crate::task::task_id::{TaskId, alloc_id};

pub trait TTaskClear {
    fn clear(&self);
}

pub struct Task<F: Future, C: TTaskClear> {
    pub tid: TaskId,
    result: RefCell<Option<F::Output>>,
    fut: Pin<Box<F>>,
    clear: C,
}

impl<F: Future, C: TTaskClear> Task<F, C> {
    pub fn new_waker(f: F, mut init_factory: impl FnMut(TaskId) -> C) -> Waker {
        let tid = alloc_id();
        let task = Rc::new(Self {
            tid: tid.clone(),
            result: RefCell::new(None),
            fut: Box::pin(f),
            clear: init_factory(tid),
        });

        Self::to_waker(Rc::into_raw(task) as *const ())
    }

    unsafe fn from_raw(data: *const ()) -> Rc<Self> {
        unsafe { Rc::from_raw(data as *const _) }
    }

    fn clone(data: *const ()) -> RawWaker {
        let task = ManuallyDrop::new(unsafe { Self::from_raw(data) });
        let task_cloned = task.deref().clone();
        Self::to_raw_waker(Rc::into_raw(task_cloned) as *const ())
    }

    fn wake(data: *const ()) {
        Self::wake_by_ref(data);
        Self::drop(data);
    }

    fn wake_by_ref(data: *const ()) {
        let task = unsafe { &mut *(data as *mut Self) };

        let waker = ManuallyDrop::new(Self::to_waker(data));
        let mut cx = Context::from_waker(&waker);
        if let Poll::Ready(result) = task.fut.as_mut().poll(&mut cx) {
            task.result.borrow_mut().replace(result);
        }
    }

    fn drop(data: *const ()) {
        let _ = unsafe { Self::from_raw(data) };
    }

    fn to_waker(data: *const ()) -> Waker {
        unsafe { Waker::from_raw(Self::to_raw_waker(data)) }
    }

    fn to_raw_waker(data: *const ()) -> RawWaker {
        RawWaker::new(
            data,
            &RawWakerVTable::new(
                Task::<F, C>::clone,
                Task::<F, C>::wake,
                Task::<F, C>::wake_by_ref,
                Task::<F, C>::drop,
            ),
        )
    }
}

impl<F: Future, C: TTaskClear> Drop for Task<F, C> {
    fn drop(&mut self) {
        self.clear.clear();
    }
}
