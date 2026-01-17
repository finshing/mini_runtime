use std::{
    cell::RefCell,
    mem::ManuallyDrop,
    ops::Deref,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use crate::runtime::task_done;

pub struct Task<F: Future> {
    result: RefCell<Option<F::Output>>,
    fut: Pin<Box<F>>,
}

impl<F: Future> Task<F> {
    pub fn new(f: F) -> Rc<Self> {
        Rc::new(Self {
            result: RefCell::new(None),
            fut: Box::pin(f),
        })
    }

    pub fn into_waker(self: Rc<Self>) -> Waker {
        Self::to_waker(Rc::into_raw(self) as *const ())
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
        let task = unsafe { Self::from_raw(data) };
        if Rc::strong_count(&task) == 1 {
            // 资源释放
            task_done(data);
        }
    }

    fn to_waker(data: *const ()) -> Waker {
        unsafe { Waker::from_raw(Self::to_raw_waker(data)) }
    }

    fn to_raw_waker(data: *const ()) -> RawWaker {
        RawWaker::new(
            data,
            &RawWakerVTable::new(
                Task::<F>::clone,
                Task::<F>::wake,
                Task::<F>::wake_by_ref,
                Task::<F>::drop,
            ),
        )
    }
}
