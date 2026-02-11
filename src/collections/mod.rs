use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub mod hash_set;

#[derive(Debug)]
pub struct ShareMutable<T>(Rc<RefCell<T>>);

impl<T> ShareMutable<T> {
    pub fn new(val: T) -> Self {
        Self(Rc::new(RefCell::new(val)))
    }

    pub fn get(&self) -> Ref<'_, T> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, T> {
        self.0.borrow_mut()
    }
}

impl<T: Default> Default for ShareMutable<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Clone for ShareMutable<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
