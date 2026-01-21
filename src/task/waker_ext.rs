use std::{borrow::Borrow, hash::Hash, task::Waker};

use crate::task::{TaskAttr, task_id::TaskId};

// Waker增强，方便被获取Waker指向Task的一些属性信息及加入到HashSet中
pub struct WakerExt(pub Waker);

impl WakerExt {
    pub fn get_task_attr(&self) -> &'static TaskAttr {
        unsafe { TaskAttr::from_raw_data(self.0.data()) }
    }
}

impl PartialEq for WakerExt {
    fn eq(&self, other: &Self) -> bool {
        self.get_task_attr().tid == other.get_task_attr().tid
    }
}

impl Eq for WakerExt {}

impl Hash for WakerExt {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get_task_attr().tid.hash(state);
    }
}

impl From<Waker> for WakerExt {
    fn from(waker: Waker) -> Self {
        Self(waker)
    }
}

// 根据HashSet::contain协议，如果要求TaskId可以作为参数，则需要实现Borrow<TaskId>
impl Borrow<TaskId> for WakerExt {
    fn borrow(&self) -> &TaskId {
        &self.get_task_attr().tid
    }
}
