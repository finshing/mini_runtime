use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub mod box_ptr_set;
pub mod hash_set;

#[derive(Debug)]
pub struct ShareMutable<T>(Rc<RefCell<T>>);

impl<T> ShareMutable<T> {
    pub fn new(val: T) -> Self {
        Self(Rc::new(RefCell::new(val)))
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.0.borrow_mut()
    }

    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
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
