pub mod task_id;
pub mod waker_ext;

use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use crate::task::task_id::{TaskId, alloc_id};

#[derive(Debug)]
// 任务属性，避免泛型带来的反解析问题
pub struct TaskAttr {
    tid: TaskId,
}

impl TaskAttr {
    fn new() -> Self {
        TaskAttr { tid: alloc_id() }
    }

    // 通过Waker::data()反解析得到任务的TaskAttr
    pub unsafe fn from_raw_data(data: *const ()) -> &'static mut Self {
        unsafe { &mut *(data as *const Self as *mut _) }
    }

    pub fn get_tid(&self) -> &TaskId {
        &self.tid
    }
}

impl Clone for TaskAttr {
    fn clone(&self) -> Self {
        Self {
            tid: self.tid.clone(),
        }
    }
}

pub trait TTaskClear {
    fn clear(&self);
}

struct TaskInner<F: Future, C: TTaskClear> {
    result: Option<F::Output>,
    fut: Pin<Box<F>>,
    clear: C,
}

impl<F: Future, C: TTaskClear> Drop for TaskInner<F, C> {
    fn drop(&mut self) {
        self.clear.clear();
    }
}

#[repr(C)]
pub struct Task<F: Future, C: TTaskClear> {
    attr: TaskAttr,
    inner: Rc<RefCell<TaskInner<F, C>>>,
}

impl<F: Future, C: TTaskClear> Task<F, C> {
    pub fn new_waker(f: F, mut init_factory: impl FnMut(TaskId) -> C) -> Waker {
        let attr = TaskAttr::new();
        let clear = init_factory(attr.tid.clone());
        let task = Box::new(Self {
            attr,
            inner: Rc::new(RefCell::new(TaskInner {
                result: None,
                fut: Box::pin(f),
                clear,
            })),
        });

        unsafe { Waker::new(Box::into_raw(task) as *const (), Self::waker_vtable()) }
    }

    /*
     * 每次clone后data都指向一个的独立的副本（虽然副本里也是各种共享指针）
     * 这样就可以对data指向的副本做独立的处理而不会影响其它对象
     */
    fn clone(data: *const ()) -> RawWaker {
        // 从Box::into_raw的源码可知获取到的就是包裹的数据在堆上的地址，因此可以直接解引用使用
        let task = unsafe { &*(data as *const Self) };
        // 需要将新副本仍然分配到堆上
        let task_cloned = Box::new(task.clone());
        // 新副本被用于clone的新对象
        RawWaker::new(
            Box::into_raw(task_cloned) as *const (),
            Self::waker_vtable(),
        )
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
        let mut task_inner = task.inner.borrow_mut();
        if let Poll::Ready(result) = task_inner.fut.as_mut().poll(&mut cx) {
            task_inner.result.replace(result);
        }
    }

    // 引用计数减一，并保证资源正确释放
    fn drop(data: *const ()) {
        let _ = unsafe { Box::from_raw(data as *mut Self) };
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

impl<F: Future, C: TTaskClear> Clone for Task<F, C> {
    fn clone(&self) -> Self {
        Self {
            attr: self.attr.clone(),
            inner: self.inner.clone(),
        }
    }
}
