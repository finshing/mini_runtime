use std::cell::{RefCell, RefMut};

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
