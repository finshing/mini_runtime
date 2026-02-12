use std::{collections::HashSet, hash::Hash};

use crate::collections::ShareMutable;

// 通过自身直接拿到句柄
pub trait ToBorrow<H>: std::borrow::Borrow<H> {
    fn to_borrow(&self) -> H;
}

// 实现Copy的类型本身就是自己的句柄
impl<T: Copy> ToBorrow<T> for T {
    fn to_borrow(&self) -> T {
        *self
    }
}

/*
 * 1. 支持随机移除——pop
 * 2. 支持定向删除——remove
 * 3. 支持生成Dropper用于随时删除
*/
pub struct HashSetExt<T: Eq + Hash>(ShareMutable<HashSet<T>>);

impl<T: Eq + Hash> HashSetExt<T> {
    pub fn add(&self, elem: T) {
        self.0.borrow_mut().insert(elem);
    }

    pub fn add_with_dropper<H>(&self, elem: T) -> HashSetExtDropper<T, H>
    where
        H: Eq + Hash,
        T: ToBorrow<H>,
    {
        let handle = elem.to_borrow();
        self.add(elem);
        HashSetExtDropper {
            set: self.clone(),
            handle,
        }
    }

    pub fn remove<H>(&self, handle: &H) -> Option<T>
    where
        H: Eq + Hash,
        T: std::borrow::Borrow<H>,
    {
        self.0.borrow_mut().take(handle)
    }

    pub fn contains<H>(&self, handle: &H) -> bool
    where
        H: Eq + Hash,
        T: std::borrow::Borrow<H>,
    {
        self.0.borrow().contains(handle)
    }

    /*
     * HashSet原生不支持pop
     */
    pub fn pop<H>(&self) -> Option<T>
    where
        H: Eq + Hash,
        T: ToBorrow<H>,
    {
        // 此处必须拿到所有权，如果拿到的是引用，会导致Ref的生命周期边长，最终和RefMut发生冲突
        let handle = self.0.borrow().iter().next().map(ToBorrow::to_borrow);

        if let Some(handle) = handle {
            self.remove(&handle)
        } else {
            None
        }
    }

    pub fn drain(&self) -> Vec<T> {
        self.0.borrow_mut().drain().collect()
    }
}

impl<T: Eq + Hash> Default for HashSetExt<T> {
    fn default() -> Self {
        Self(ShareMutable::new(HashSet::new()))
    }
}

impl<T: Eq + Hash> Clone for HashSetExt<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct HashSetExtDropper<T, H>
where
    T: Eq + Hash,
    H: Eq + Hash,
    T: std::borrow::Borrow<H>,
{
    set: HashSetExt<T>,
    handle: H,
}

impl<T, H> Drop for HashSetExtDropper<T, H>
where
    T: Eq + Hash,
    H: Eq + Hash,
    T: std::borrow::Borrow<H>,
{
    fn drop(&mut self) {
        self.set.remove(&self.handle);
    }
}

#[cfg(test)]
mod test {
    use crate::collections::hash_set::{HashSetExt, ToBorrow};

    #[derive(PartialEq, Eq, Hash, Debug)]
    struct Int(usize);

    impl std::borrow::Borrow<usize> for Int {
        fn borrow(&self) -> &usize {
            &self.0
        }
    }

    impl ToBorrow<usize> for Int {
        fn to_borrow(&self) -> usize {
            self.0
        }
    }

    #[test]
    fn test_hash_set_ext() {
        let set = HashSetExt::default();

        set.add(Int(0));
        set.add(Int(1));
        set.add(Int(2));

        while let Some(elem) = set.pop() {
            println!("elem: {elem:?}");
        }

        assert_eq!(set.drain().len(), 0);

        {
            let _dropper = set.add_with_dropper(Int(0));
            println!("contain 0: {}-{}", set.contains(&0), set.contains(&Int(0)));
        }

        println!("contain 0: {}-{}", set.contains(&0), set.contains(&Int(0)));
    }
}
