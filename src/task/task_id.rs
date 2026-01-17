use std::{fmt::Display, rc::Rc};

use lazy_static::lazy_static;

use crate::helper::UPSafeCell;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct _TaskId(usize);

impl Drop for _TaskId {
    fn drop(&mut self) {
        TASK_ID_GEN.get_mut().recycle_use(self.0);
    }
}

impl Display for _TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task={}", self.0)
    }
}

pub type TaskId = Rc<_TaskId>;

/*
* task_id生成器，可以回收利用过期的task_id
*/
#[derive(Default)]
struct TaskIdGenerator {
    cur: usize,
    pool: Vec<usize>,
}

impl TaskIdGenerator {
    fn gen_id(&mut self) -> TaskId {
        let tid = if let Some(tid) = self.pool.pop() {
            tid
        } else {
            self.cur += 1;
            self.cur - 1
        };

        Rc::new(_TaskId(tid))
    }

    fn recycle_use(&mut self, id: usize) {
        self.pool.push(id);
    }
}

lazy_static! {
    static ref TASK_ID_GEN: UPSafeCell<TaskIdGenerator> =
        UPSafeCell::new(TaskIdGenerator::default());
}

pub(crate) fn alloc_id() -> TaskId {
    TASK_ID_GEN.get_mut().gen_id()
}
