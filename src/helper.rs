use std::{
    cell::{RefCell, RefMut},
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll},
};

use crate::runtime::add_waker;

pub struct UPSafeCell<T> {
    inner: RefCell<T>,
}

impl<T> UPSafeCell<T> {
    pub fn new(t: T) -> Self {
        Self {
            inner: RefCell::new(t),
        }
    }

    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}

unsafe impl<T> Send for UPSafeCell<T> {}

unsafe impl<T> Sync for UPSafeCell<T> {}

/// 取出素银at之前的数据，并保留其后的数据
/// let mut a = vec![1, 2, 3, 4];
/// let b = take_vec_at(&mut a, 2);
/// assert_eq!(a, vec![3, 4]);
/// assert_eq!(b, vec![1, 2]);
pub fn take_vec_at<T>(v: &mut Vec<T>, at: usize) -> Vec<T> {
    if at > v.len() {
        return Vec::new();
    }

    let left = v.split_off(at);
    std::mem::replace(v, left)
}

pub fn poll_fn<T, F>(f: F) -> PollFn<T, F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    PollFn { poller: f }
}

pub struct PollFn<T, F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    poller: F,
}

impl<T, F> Future for PollFn<T, F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    type Output = T;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        (unsafe { &mut self.get_unchecked_mut().poller })(cx)
    }
}

pub enum FutureResult<T> {
    Taken,
    Done(T),
}

/// 部分场景下Future::poll()在返回Poll::Ready后，再次被调用可能返回Poll::Pending（依赖于业务侧的实现）
/// 为了保证Future::poll()在第一次返回Poll::Ready之后的调用中每次都返回Poll::Ready，这里对Future做一次扩展
pub struct FutureExt<F: Future> {
    f: Pin<Box<F>>,
    finished: bool,
}

impl<F: Future> FutureExt<F> {
    pub fn new(f: F) -> Self {
        Self {
            f: Box::pin(f),
            finished: false,
        }
    }

    pub fn new_with_result_placeholder(f: F) -> (Self, MaybeUninit<F::Output>) {
        (Self::new(f), MaybeUninit::uninit())
    }
}

impl<F: Future> Future for FutureExt<F> {
    type Output = FutureResult<F::Output>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.finished {
            return Poll::Ready(FutureResult::Taken);
        }

        if let Poll::Ready(result) = self.f.as_mut().poll(cx) {
            self.finished = true;
            return Poll::Ready(FutureResult::Done(result));
        }

        Poll::Pending
    }
}

#[allow(unused)]
pub fn yield_here() -> Yield {
    Yield { yielded: false }
}

pub struct Yield {
    yielded: bool,
}

impl Future for Yield {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.get_mut().yielded = true;
            add_waker(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::helper::take_vec_at;

    #[test]
    fn test_take_vec_at() {
        let mut a = vec![1, 2, 3, 4];
        let b = take_vec_at(&mut a, 2);
        assert_eq!(a, vec![3, 4]);
        assert_eq!(b, vec![1, 2]);
    }
}
