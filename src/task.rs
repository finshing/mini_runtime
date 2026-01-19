use std::{
    cell::RefCell,
    mem::ManuallyDrop,
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
    // 通过Rc保证Task被存储在堆上，并可以使用引用计数来保证资源的正确释放
    pub fn new_waker(f: F) -> Waker {
        let task = Rc::new(Self {
            result: RefCell::new(None),
            fut: Box::pin(f),
        });

        unsafe {
            Waker::from_raw(RawWaker::new(
                Rc::into_raw(task) as *const (),
                Self::waker_vtable(),
            ))
        }
    }

    // 引用计数加一
    fn clone(data: *const ()) -> RawWaker {
        let task = ManuallyDrop::new(unsafe { Rc::from_raw(data as *const Self) });
        let _task_cloned = task.clone();
        RawWaker::new(data, Self::waker_vtable())
    }

    // 需要保证自身被消费掉
    fn wake(data: *const ()) {
        Self::wake_by_ref(data);
        Self::drop(data);
    }

    fn wake_by_ref(data: *const ()) {
        let task = unsafe { &mut *(data as *mut Self) };

        // 构造cx，并通过Future::poll驱动异步的执行
        // 这虽然用到了clone，但之后又会被drop，因此可以保证Rc<Task<F>>的引用计数不发生变化
        let waker = unsafe { Waker::from_raw(Self::clone(data)) };
        let mut cx = Context::from_waker(&waker);
        if let Poll::Ready(result) = task.fut.as_mut().poll(&mut cx) {
            task.result.borrow_mut().replace(result);
        }
    }

    // 引用计数减一，并保证资源正确释放
    fn drop(data: *const ()) {
        let task = unsafe { Rc::from_raw(data as *const Self) };
        if Rc::strong_count(&task) == 1 {
            // 资源释放。（在后边的Runtime中提供实现）
            task_done(data);
        }
    }

    fn waker_vtable() -> &'static RawWakerVTable {
        &RawWakerVTable::new(
            Task::<F>::clone,
            Task::<F>::wake,
            Task::<F>::wake_by_ref,
            Task::<F>::drop,
        )
    }
}
