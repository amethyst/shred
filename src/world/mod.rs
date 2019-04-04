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

/// The id of a [`Resource`], which simply wraps a type id and a "dynamic ID".
/// The "dynamic ID" is usually just left `0`, and, unless such documentation
/// says otherwise, other libraries will assume that it is always `0`; non-zero
/// IDs are only used for special resource types that are specifically defined
/// in a more dynamic way, such that resource types can essentially be created
/// at run time, without having different static types.
///
/// [`Resource`]: trait.Resource.html
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceId {
    type_id: TypeId,
    dynamic_id: u64,
}

impl ResourceId {
    /// Creates a new resource id from a given type.
    #[inline]
    pub fn new<T: Resource>() -> Self {
        ResourceId::new_with_dynamic_id::<T>(0)
    }

    /// Create a new resource id from a raw type ID.
    #[inline]
    pub fn from_type_id(type_id: TypeId) -> Self {
        ResourceId::from_type_id_and_dynamic_id(type_id, 0)
    }

    /// Creates a new resource id from a given type and a `dynamic_id`.
    ///
    /// This is usually not what you want (unless you're implementing scripting
    /// with `shred` or some similar mechanism to define resources at run-time).
    ///
    /// Creating resource IDs with a `dynamic_id` unequal to `0` is only
    /// recommended for special types that are specifically defined for
    /// scripting; most libraries will just assume that resources are
    /// identified only by their type.
    #[inline]
    pub fn new_with_dynamic_id<T: Resource>(dynamic_id: u64) -> Self {
        ResourceId::from_type_id_and_dynamic_id(TypeId::of::<T>(), dynamic_id)
    }

    /// Create a new resource id from a raw type ID and a "dynamic ID" (see type
    /// documentation).
    #[inline]
    pub fn from_type_id_and_dynamic_id(type_id: TypeId, dynamic_id: u64) -> Self {
        ResourceId {
            type_id,
            dynamic_id,
        }
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
        self.insert_internal(ResourceId::new::<R>(), r);
    }

    /// Removes a resource of type `R` from the `World` and returns its
    /// ownership to the caller. In case there is no such resource in this
    /// `World`, `None` will be returned.
    ///
    /// Use this method with caution; other functions and systems might assume
    /// this resource still exists. Thus, only use this if you're sure no
    /// system will try to access this resource after you removed it (or else
    /// you will get a panic).
    pub fn remove<R>(&mut self) -> Option<R>
    where
        R: Resource,
    {
        self.remove_internal(ResourceId::new::<R>())
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
        self.try_fetch().unwrap_or_else(|| {
            if self.resources.is_empty() {
                eprintln!(
                    "Note: Could not find a resource (see the following panic);\
                     the `World` is completely empty. Did you accidentally create a fresh `World`?"
                )
            }

            fetch_panic!()
        })
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

    /// Like `try_fetch`, but fetches the resource by its `ResourceId` which
    /// allows using a dynamic ID.
    ///
    /// This is usually not what you need; please read the type-level
    /// documentation of `ResourceId`.
    ///
    /// # Panics
    ///
    /// This method panics if `id` refers to a different type ID than `T`.
    pub fn try_fetch_by_id<T>(&self, id: ResourceId) -> Option<Fetch<T>>
    where
        T: Resource,
    {
        let res_id0 = ResourceId::new::<T>();
        assert_eq!(
            res_id0.type_id, id.type_id,
            "Passed a `ResourceId` with a wrong type ID"
        );

        self.resources.get(&id).map(|r| Fetch {
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

    /// Like `try_fetch_mut`, but fetches the resource by its `ResourceId` which
    /// allows using a dynamic ID.
    ///
    /// This is usually not what you need; please read the type-level
    /// documentation of `ResourceId`.
    ///
    /// # Panics
    ///
    /// This method panics if `id` refers to a different type ID than `T`.
    pub fn try_fetch_mut_by_id<T>(&self, id: ResourceId) -> Option<FetchMut<T>>
    where
        T: Resource,
    {
        let res_id0 = ResourceId::new::<T>();
        assert_eq!(
            res_id0.type_id, id.type_id,
            "Passed a `ResourceId` with a wrong type ID"
        );

        self.resources.get(&id).map(|r| FetchMut {
            inner: r.borrow_mut(),
            phantom: PhantomData,
        })
    }

    /// Internal function for inserting resources, should only be used if you
    /// know what you're doing.
    ///
    /// This is useful for inserting resources with a custom `ResourceId`.
    pub fn insert_internal<R>(&mut self, id: ResourceId, r: R)
    where
        R: Resource,
    {
        self.resources.insert(id, TrustCell::new(Box::new(r)));
    }

    /// Internal function for removing resources, should only be used if you
    /// know what you're doing.
    ///
    /// This is useful for removing resources with a custom `ResourceId`.
    pub fn remove_internal<R>(&mut self, id: ResourceId) -> Option<R>
    where
        R: Resource,
    {
        self.resources
            .remove(&id)
            .map(TrustCell::into_inner)
            .map(|x: Box<Resource>| x.downcast())
            .map(|x: Result<Box<R>, _>| x.ok().unwrap())
            .map(|x| *x)
    }

    /// Internal function for fetching resources, should only be used if you
    /// know what you're doing.
    pub fn try_fetch_internal(&self, id: ResourceId) -> Option<&TrustCell<Box<Resource>>> {
        self.resources.get(&id)
    }

    /// Retrieves a resource without fetching, which is cheaper, but only
    /// available with `&mut self`.
    pub fn get_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.get_mut_raw(ResourceId::new::<T>())
            .map(|res| unsafe { res.downcast_mut_unchecked() })
    }

    /// Retrieves a resource without fetching, which is cheaper, but only
    /// available with `&mut self`.
    pub fn get_mut_raw(&mut self, id: ResourceId) -> Option<&mut Resource> {
        self.resources
            .get_mut(&id)
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

        let mut world = World::new();
        world.insert(Res);
        <Read<Res> as SystemData>::fetch(&world);
    }

    #[test]
    fn fetch_mut_aspects() {
        assert_eq!(Write::<Res>::reads(), vec![]);
        assert_eq!(Write::<Res>::writes(), vec![ResourceId::new::<Res>()]);

        let mut world = World::new();
        world.insert(Res);
        <Write<Res> as SystemData>::fetch(&world);
    }

    #[test]
    fn fetch_by_id() {
        let mut world = World::new();

        world.insert_internal(ResourceId::new_with_dynamic_id::<i32>(1), 5);
        world.insert_internal(ResourceId::new_with_dynamic_id::<i32>(2), 15);
        world.insert_internal(ResourceId::new_with_dynamic_id::<i32>(3), 45);

        assert_eq!(
            world
                .try_fetch_by_id::<i32>(ResourceId::new_with_dynamic_id::<i32>(2))
                .map(|x| *x),
            Some(15)
        );
        assert_eq!(
            world
                .try_fetch_by_id::<i32>(ResourceId::new_with_dynamic_id::<i32>(1))
                .map(|x| *x),
            Some(5)
        );
        assert_eq!(
            world
                .try_fetch_by_id::<i32>(ResourceId::new_with_dynamic_id::<i32>(3))
                .map(|x| *x),
            Some(45)
        );
    }

    #[test]
    #[should_panic]
    fn invalid_fetch_by_id0() {
        let mut world = World::new();

        world.insert(5i32);

        world.try_fetch_by_id::<u32>(ResourceId::new_with_dynamic_id::<i32>(111));
    }

    #[test]
    #[should_panic]
    fn invalid_fetch_by_id1() {
        let mut world = World::new();

        world.insert(5i32);

        world.try_fetch_by_id::<i32>(ResourceId::new_with_dynamic_id::<u32>(111));
    }

    #[test]
    fn add() {
        struct Foo;

        let mut world = World::new();
        world.insert(Res);

        assert!(world.has_value::<Res>());
        assert!(!world.has_value::<Foo>());
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed")]
    fn read_write_fails() {
        let mut world = World::new();
        world.insert(Res);

        let read: Fetch<Res> = world.fetch();
        let write: FetchMut<Res> = world.fetch_mut();
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed mutably")]
    fn write_read_fails() {
        let mut world = World::new();
        world.insert(Res);

        let write: FetchMut<Res> = world.fetch_mut();
        let read: Fetch<Res> = world.fetch();
    }

    #[test]
    fn remove_insert() {
        let mut world = World::new();

        world.insert(Res);

        assert!(world.has_value::<Res>());

        println!("{:#?}", world.resources.keys().collect::<Vec<_>>());

        world.remove::<Res>().unwrap();

        assert!(!world.has_value::<Res>());

        world.insert(Res);

        assert!(world.has_value::<Res>());
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

        let mut world = World::new();
        assert!(world.try_fetch::<i32>().is_none());

        let mut sys = Sys;
        RunNow::setup(&mut sys, &mut world);

        sys.run_now(&world);

        assert!(world.try_fetch::<i32>().is_some());
        assert_eq!(*world.fetch::<i32>(), 33);
    }
}
