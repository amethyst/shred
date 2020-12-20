use std::marker::PhantomData;

use crate::{
    cell::{RefMut, TrustCell},
    world::{FetchMut, Resource, ResourceId},
};

type StdEntry<'a, K, V> =
    hashbrown::hash_map::Entry<'a, K, V, hashbrown::hash_map::DefaultHashBuilder>;

/// An entry to a resource of the `World` struct.
/// This is similar to the Entry API found in the standard library.
///
/// ## Examples
///
/// ```
/// use shred::World;
///
/// #[derive(Debug)]
/// struct Res(i32);
///
/// let mut world = World::empty();
///
/// let value = world.entry().or_insert(Res(4));
/// println!("{:?}", value.0 * 2);
/// ```
pub struct Entry<'a, T: 'a> {
    inner: StdEntry<'a, ResourceId, TrustCell<Box<dyn Resource>>>,
    marker: PhantomData<T>,
}

impl<'a, T> Entry<'a, T>
where
    T: Resource + 'a,
{
    /// Returns this entry's value, inserts and returns `v` otherwise.
    ///
    /// Please note that you should use `or_insert_with` in case the creation of
    /// the value is expensive.
    pub fn or_insert(self, v: T) -> FetchMut<'a, T> {
        self.or_insert_with(move || v)
    }

    /// Returns this entry's value, inserts and returns the return value of `f`
    /// otherwise.
    pub fn or_insert_with<F>(self, f: F) -> FetchMut<'a, T>
    where
        F: FnOnce() -> T,
    {
        let value = self
            .inner
            .or_insert_with(move || TrustCell::new(Box::new(f())));
        let inner = RefMut::map(value.borrow_mut(), Box::as_mut);

        FetchMut {
            inner,
            phantom: PhantomData,
        }
    }
}

pub fn create_entry<T>(e: StdEntry<ResourceId, TrustCell<Box<dyn Resource>>>) -> Entry<T> {
    Entry {
        inner: e,
        marker: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use crate::world::World;

    #[test]
    fn test_entry() {
        struct Res;

        let mut world = World::empty();
        world.entry().or_insert(Res);

        assert!(world.has_value::<Res>());
    }
}
