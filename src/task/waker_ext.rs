use std::{
    cell::RefCell,
    collections::HashSet,
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
    task::Waker,
};

use crate::task::{TaskAttr, task_id::TaskId};

#[derive(Default, Debug)]
pub struct WakerSet(Rc<RefCell<HashSet<WakerExt>>>);

impl WakerSet {
    pub fn add_waker(&mut self, waker: Waker) {
        self.0.borrow_mut().insert(waker.into());
    }

    #[allow(unused)]
    pub fn remove_waker<T>(&mut self, handle: &T)
    where
        T: Eq + Hash,
        WakerExt: std::borrow::Borrow<T>,
    {
        self.0.borrow_mut().remove(handle);
    }

    pub fn contains<T>(&self, handle: &T) -> bool
    where
        T: Eq + Hash,
        WakerExt: std::borrow::Borrow<T>,
    {
        self.0.borrow().contains(handle)
    }

    pub fn drain(&mut self) -> Vec<Waker> {
        self.0
            .borrow_mut()
            .drain()
            .map(|waker_ext| waker_ext.0)
            .collect()
    }

    pub fn pop(&mut self) -> Option<Waker> {
        let tid = self
            .0
            .borrow()
            .iter()
            .next()
            .map(|waker_ext| waker_ext.tid.clone());

        if let Some(tid) = tid {
            self.0.borrow_mut().take(&tid).map(|waker_ext| waker_ext.0)
        } else {
            None
        }
    }
}

// Waker增强，方便被获取Waker指向Task的一些属性信息及加入到HashSet中
pub struct WakerExt(pub Waker);

impl Deref for WakerExt {
    type Target = TaskAttr;

    fn deref(&self) -> &Self::Target {
        unsafe { TaskAttr::from_raw_data(self.0.data()) }
    }
}

impl DerefMut for WakerExt {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { TaskAttr::from_raw_data(self.0.data()) }
    }
}

impl PartialEq for WakerExt {
    fn eq(&self, other: &Self) -> bool {
        self.tid == other.tid
    }
}

impl Eq for WakerExt {}

impl Hash for WakerExt {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tid.hash(state);
    }
}

impl From<Waker> for WakerExt {
    fn from(waker: Waker) -> Self {
        Self(waker)
    }
}

// 根据HashSet::contain协议，如果要求TaskId可以作为参数，则需要实现Borrow<TaskId>
impl std::borrow::Borrow<TaskId> for WakerExt {
    fn borrow(&self) -> &TaskId {
        &self.tid
    }
}

impl Debug for WakerExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WakerExt{{ {:?} }}", self.deref())
    }
}
