pub mod task_id;
pub mod waker_ext;

use std::{
    cell::RefCell,
    mem::ManuallyDrop,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use crate::task::task_id::{TaskId, alloc_id};

#[derive(Debug)]
// 任务属性，避免泛型带来的反解析问题
pub struct TaskAttr {
    pub tid: TaskId,
}

impl TaskAttr {
    fn new() -> Self {
        TaskAttr { tid: alloc_id() }
    }

    // 通过Waker::data()反解析得到任务的TaskAttr
    pub unsafe fn from_raw_data(data: *const ()) -> &'static mut Self {
        unsafe { &mut *(data as *const Self as *mut _) }
    }
}

pub trait TTaskClear {
    fn clear(&self);
}

#[repr(C)]
pub struct Task<F: Future, C: TTaskClear> {
    attr: TaskAttr,
    result: RefCell<Option<F::Output>>,
    fut: Pin<Box<F>>,
    clear: C,
}

impl<F: Future, C: TTaskClear> Task<F, C> {
    pub fn new_waker(f: F, mut init_factory: impl FnMut(TaskId) -> C) -> Waker {
        let attr = TaskAttr::new();
        let clear = init_factory(attr.tid.clone());
        let task = Rc::new(Self {
            attr,
            result: RefCell::new(None),
            fut: Box::pin(f),
            clear,
        });

        unsafe { Waker::new(Rc::into_raw(task) as *const (), Self::waker_vtable()) }
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
        let _ = unsafe { Rc::from_raw(data as *const Self) };
    }

    fn waker_vtable() -> &'static RawWakerVTable {
        &RawWakerVTable::new(
            Task::<F, C>::clone,
            Task::<F, C>::wake,
            Task::<F, C>::wake_by_ref,
            Task::<F, C>::drop,
        )
    }
}

impl<F: Future, C: TTaskClear> Drop for Task<F, C> {
    fn drop(&mut self) {
        self.clear.clear();
    }
}
