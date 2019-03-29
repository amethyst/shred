//! Module for resource related types

pub use self::{
    data::{Read, ReadExpect, Write, WriteExpect},
    entry::Entry,
    setup::{DefaultProvider, PanicHandler, SetupHandler},
};

use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use hashbrown::HashMap;
use mopa::Any;

use crate::cell::{Ref, RefMut, TrustCell};

use self::entry::create_entry;

mod data;
mod entry;
#[macro_use]
mod setup;

/// Allows to fetch a resource in a system immutably.
///
/// If the resource isn't strictly required, you should use `Option<Fetch<T>>`.
///
/// # Type parameters
///
/// * `T`: The type of the resource
pub struct Fetch<'a, T: 'a> {
    inner: Ref<'a, Box<Resource>>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> Deref for Fetch<'a, T>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.inner.downcast_ref_unchecked() }
    }
}

/// Allows to fetch a resource in a system mutably.
///
/// If the resource isn't strictly required, you should use
/// `Option<FetchMut<T>>`.
///
/// # Type parameters
///
/// * `T`: The type of the resource
pub struct FetchMut<'a, T: 'a> {
    inner: RefMut<'a, Box<Resource>>,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> Deref for FetchMut<'a, T>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.inner.downcast_ref_unchecked() }
    }
}

impl<'a, T> DerefMut for FetchMut<'a, T>
where
    T: Resource,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.inner.downcast_mut_unchecked() }
    }
}

/// A resource is a data slot which lives in the `World` can only be accessed
/// according to Rust's typical borrowing model (one writer xor multiple
/// readers).
pub trait Resource: Any + Send + Sync + 'static {}

mopafy!(Resource);

impl<T> Resource for T where T: Any + Send + Sync {}

/// The id of a [`Resource`], which simply wraps a type id.
///
/// [`Resource`]: trait.Resource.html
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceId(pub TypeId);

impl ResourceId {
    /// Creates a new resource id from a given type.
    pub fn new<T: Resource>() -> Self {
        ResourceId(TypeId::of::<T>())
    }
}

/// A resource container, which provides methods to access to
/// the contained resources.
///
/// # Resource Ids
///
/// Resources are identified by `ResourceId`s, which consist of a `TypeId`.
#[derive(Default)]
pub struct World {
    resources: HashMap<ResourceId, TrustCell<Box<Resource>>>,
}

impl World {
    /// Creates a new, empty resource container.
    pub fn new() -> Self {
        Default::default()
    }

    /// Inserts a resource into this container. If the resource existed before,
    /// it will be overwritten.
    ///
    /// # Examples
    ///
    /// Every type satisfying `Any + Debug + Send + Sync` automatically
    /// implements `Resource`, thus can be added:
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// #[derive(Debug)]
    /// struct MyRes(i32);
    /// ```
    ///
    /// When you have a resource, simply insert it like this:
    ///
    /// ```rust
    /// # #[derive(Debug)] struct MyRes(i32);
    /// use shred::World;
    ///
    /// let mut world = World::new();
    /// world.insert(MyRes(5));
    /// ```
    pub fn insert<R>(&mut self, r: R)
    where
        R: Resource,
    {
        self.resources
            .insert(ResourceId::new::<R>(), TrustCell::new(Box::new(r)));
    }

    /// Returns true if the specified resource type `R` exists in `self`.
    pub fn has_value<R>(&self) -> bool
    where
        R: Resource,
    {
        self.has_value_raw(ResourceId::new::<R>())
    }

    /// Returns true if the specified resource type exists in `self`.
    pub fn has_value_raw(&self, id: ResourceId) -> bool {
        self.resources.contains_key(&id)
    }

    /// Returns an entry for the resource with type `R`.
    pub fn entry<R>(&mut self) -> Entry<R>
    where
        R: Resource,
    {
        create_entry(self.resources.entry(ResourceId::new::<R>()))
    }

    /// Fetches the resource with the specified type `T` or panics if it doesn't
    /// exist.
    ///
    /// # Panics
    ///
    /// Panics if the resource doesn't exist.
    /// Panics if the resource is being accessed mutably.
    pub fn fetch<T>(&self) -> Fetch<T>
    where
        T: Resource,
    {
        self.try_fetch().unwrap_or_else(|| fetch_panic!())
    }

    /// Like `fetch`, but returns an `Option` instead of inserting a default
    /// value in case the resource does not exist.
    pub fn try_fetch<T>(&self) -> Option<Fetch<T>>
    where
        T: Resource,
    {
        let res_id = ResourceId::new::<T>();

        self.resources.get(&res_id).map(|r| Fetch {
            inner: r.borrow(),
            phantom: PhantomData,
        })
    }

    /// Fetches the resource with the specified type `T` mutably.
    ///
    /// Please see `fetch` for details.
    ///
    /// # Panics
    ///
    /// Panics if the resource doesn't exist.
    /// Panics if the resource is already being accessed.
    pub fn fetch_mut<T>(&self) -> FetchMut<T>
    where
        T: Resource,
    {
        self.try_fetch_mut().unwrap_or_else(|| fetch_panic!())
    }

    /// Like `fetch_mut`, but returns an `Option` instead of inserting a default
    /// value in case the resource does not exist.
    pub fn try_fetch_mut<T>(&self) -> Option<FetchMut<T>>
    where
        T: Resource,
    {
        let res_id = ResourceId::new::<T>();

        self.resources.get(&res_id).map(|r| FetchMut {
            inner: r.borrow_mut(),
            phantom: PhantomData,
        })
    }

    /// Internal function for fetching resources, should only be used if you
    /// know what you're doing.
    pub fn try_fetch_internal(&self, id: TypeId) -> Option<&TrustCell<Box<Resource>>> {
        self.resources.get(&ResourceId(id))
    }

    /// Retrieves a resource without fetching, which is cheaper, but only
    /// available with `&mut self`.
    pub fn get_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.get_mut_raw(TypeId::of::<T>())
            .map(|res| unsafe { res.downcast_mut_unchecked() })
    }

    /// Retrieves a resource without fetching, which is cheaper, but only
    /// available with `&mut self`.
    pub fn get_mut_raw(&mut self, id: TypeId) -> Option<&mut Resource> {
        self.resources
            .get_mut(&ResourceId(id))
            .map(TrustCell::get_mut)
            .map(Box::as_mut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunNow, System, SystemData};

    #[derive(Default)]
    struct Res;

    #[test]
    fn fetch_aspects() {
        assert_eq!(Read::<Res>::reads(), vec![ResourceId::new::<Res>()]);
        assert_eq!(Read::<Res>::writes(), vec![]);

        let mut res = World::new();
        res.insert(Res);
        <Read<Res> as SystemData>::fetch(&res);
    }

    #[test]
    fn fetch_mut_aspects() {
        assert_eq!(Write::<Res>::reads(), vec![]);
        assert_eq!(Write::<Res>::writes(), vec![ResourceId::new::<Res>()]);

        let mut res = World::new();
        res.insert(Res);
        <Write<Res> as SystemData>::fetch(&res);
    }

    #[test]
    fn add() {
        struct Foo;

        let mut res = World::new();
        res.insert(Res);

        assert!(res.has_value::<Res>());
        assert!(!res.has_value::<Foo>());
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed")]
    fn read_write_fails() {
        let mut res = World::new();
        res.insert(Res);

        let read: Fetch<Res> = res.fetch();
        let write: FetchMut<Res> = res.fetch_mut();
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed mutably")]
    fn write_read_fails() {
        let mut res = World::new();
        res.insert(Res);

        let write: FetchMut<Res> = res.fetch_mut();
        let read: Fetch<Res> = res.fetch();
    }

    #[test]
    fn default_works() {
        struct Sys;

        impl<'a> System<'a> for Sys {
            type SystemData = Write<'a, i32>;

            fn run(&mut self, mut data: Self::SystemData) {
                assert_eq!(*data, 0);

                *data = 33;
            }
        }

        let mut res = World::new();
        assert!(res.try_fetch::<i32>().is_none());

        let mut sys = Sys;
        RunNow::setup(&mut sys, &mut res);

        sys.run_now(&res);

        assert!(res.try_fetch::<i32>().is_some());
        assert_eq!(*res.fetch::<i32>(), 33);
    }
}
