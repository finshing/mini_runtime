use std::collections::HashMap;

use crate::collections::ShareMutable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetPtr(*const ());

impl From<*const ()> for SetPtr {
    fn from(ptr: *const ()) -> Self {
        SetPtr(ptr)
    }
}

/*
 * 能够存储对象，并能快速O(1)删除
 */
#[derive(Debug)]
pub struct BoxPtrSet<T> {
    inner: ShareMutable<HashMap<SetPtr, Box<T>>>,
}

impl<T> BoxPtrSet<T> {
    pub fn insert(&self, val: T) -> SetPtr {
        let val = Box::new(val);
        let ptr = val.as_ref() as *const T as *const ();
        self.inner.borrow_mut().insert(ptr.into(), val);
        ptr.into()
    }

    pub fn build_dropper(&self, ptr: SetPtr) -> BoxPtrSetDropper<T> {
        BoxPtrSetDropper {
            set: self.clone(),
            ptr,
        }
    }

    pub fn contains(&self, ptr: &SetPtr) -> bool {
        self.inner.borrow().contains_key(ptr)
    }

    pub fn remove(&self, ptr: &SetPtr) -> Option<T> {
        self.inner.borrow_mut().remove(ptr).map(|val| *val)
    }
}

impl<T> Clone for BoxPtrSet<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Default for BoxPtrSet<T> {
    fn default() -> Self {
        Self {
            inner: ShareMutable::default(),
        }
    }
}

pub struct BoxPtrSetDropper<T> {
    set: BoxPtrSet<T>,
    ptr: SetPtr,
}

impl<T> Drop for BoxPtrSetDropper<T> {
    fn drop(&mut self) {
        self.set.remove(&self.ptr);
    }
}
