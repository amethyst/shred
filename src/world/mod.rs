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

use crate::{
    cell::{Ref, RefMut, TrustCell},
    SystemData,
};

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
    inner: Ref<'a, dyn Resource>,
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

impl<'a, T> Clone for Fetch<'a, T> {
    fn clone(&self) -> Self {
        Fetch {
            inner: self.inner.clone(),
            phantom: PhantomData,
        }
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
    inner: RefMut<'a, dyn Resource>,
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
#[cfg(feature = "parallel")]
pub trait Resource: Any + Send + Sync + 'static {}

/// A resource is a data slot which lives in the `World` can only be accessed
/// according to Rust's typical borrowing model (one writer xor multiple
/// readers).
#[cfg(not(feature = "parallel"))]
pub trait Resource: Any + 'static {}

mod __resource_mopafy_scope {
    #![allow(clippy::all)]

    use mopa::mopafy;

    use super::Resource;

    mopafy!(Resource);
}

#[cfg(feature = "parallel")]
impl<T> Resource for T where T: Any + Send + Sync {}
#[cfg(not(feature = "parallel"))]
impl<T> Resource for T where T: Any {}

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

    fn assert_same_type_id<R: Resource>(&self) {
        let res_id0 = ResourceId::new::<R>();
        assert_eq!(
            res_id0.type_id, self.type_id,
            "Passed a `ResourceId` with a wrong type ID"
        );
    }
}

/// A [Resource] container, which provides methods to insert, access and manage
/// the contained resources.
///
/// Many methods take `&self` which works because everything
/// is stored with **interior mutability**. In case you violate
/// the borrowing rules of Rust (multiple reads xor one write),
/// you will get a panic.
///
/// # Use with Specs
///
/// If you're using this from the Specs ECS library, there are two things to be
/// aware of:
///
/// 1. There are many utility methods Specs provides. To use them, you need to
/// import `specs::WorldExt`.
/// 2. You should not use [World::empty], but rather `specs::WorldExt::new`. The
/// latter can simply be called using `World::new()`, as long as `WorldExt`
/// is imported.
///
/// # Resource Ids
///
/// Resources are identified by `ResourceId`s, which consist of a `TypeId`.
#[derive(Default)]
pub struct World {
    resources: HashMap<ResourceId, TrustCell<Box<dyn Resource>>>,
}

impl World {
    /// Creates a new, empty resource container.
    ///
    /// Note that if you're using Specs, you should use `WorldExt::new` instead.
    pub fn empty() -> Self {
        Default::default()
    }

    /// Inserts a resource into this container. If the resource existed before,
    /// it will be overwritten.
    ///
    /// # Examples
    ///
    /// Every type satisfying `Any + Send + Sync` automatically
    /// implements `Resource`, thus can be added:
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// struct MyRes(i32);
    /// ```
    ///
    /// When you have a resource, simply insert it like this:
    ///
    /// ```rust
    /// # struct MyRes(i32);
    /// use shred::World;
    ///
    /// let mut world = World::empty();
    /// world.insert(MyRes(5));
    /// ```
    pub fn insert<R>(&mut self, r: R)
    where
        R: Resource,
    {
        self.insert_by_id(ResourceId::new::<R>(), r);
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
        self.remove_by_id(ResourceId::new::<R>())
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

    /// Gets `SystemData` `T` from the `World`. This can be used to retrieve
    /// data just like in [System](crate::System)s.
    ///
    /// This will not setup the system data, i.e. resources fetched here must
    /// exist already.
    ///
    /// # Examples
    ///
    /// ```
    /// # use shred::*;
    /// # #[derive(Default)] struct Timer; #[derive(Default)] struct AnotherResource;
    ///
    /// // NOTE: If you use Specs, use `World::new` instead.
    /// let mut world = World::empty();
    /// world.insert(Timer);
    /// world.insert(AnotherResource);
    /// let system_data: (Read<Timer>, Read<AnotherResource>) = world.system_data();
    /// ```
    ///
    /// # Panics
    ///
    /// * Panics if `T` is already borrowed in an incompatible way.
    pub fn system_data<'a, T>(&'a self) -> T
    where
        T: SystemData<'a>,
    {
        SystemData::fetch(&self)
    }

    /// Sets up system data `T` for fetching afterwards.
    ///
    /// Most `SystemData` implementations will insert a sensible default value,
    /// by implementing [SystemData::setup]. However, it is not guaranteed to
    /// do that; if there is no sensible default, `setup` might not do anything.
    ///
    /// # Examples
    ///
    /// ```
    /// use shred::{Read, World};
    ///
    /// #[derive(Default)]
    /// struct MyCounter(u32);
    ///
    /// // NOTE: If you use Specs, use `World::new` instead.
    /// let mut world = World::empty();
    /// assert!(!world.has_value::<MyCounter>());
    ///
    /// // `Read<MyCounter>` requires a `Default` implementation, and uses
    /// // that to initialize the resource
    /// world.setup::<Read<MyCounter>>();
    /// assert!(world.has_value::<MyCounter>());
    /// ```
    ///
    /// Here's another example, showing the case where no resource gets
    /// initialized:
    ///
    /// ```
    /// use shred::{ReadExpect, World};
    ///
    /// struct MyCounter(u32);
    ///
    /// // NOTE: If you use Specs, use `World::new` instead.
    /// let mut world = World::empty();
    ///
    /// world.setup::<ReadExpect<MyCounter>>();
    /// ```
    pub fn setup<'a, T: SystemData<'a>>(&mut self) {
        T::setup(self);
    }

    /// Executes `f` once, right now and with the specified system data.
    ///
    /// This sets up the system data `f` expects, fetches it and then
    /// executes `f`. This is essentially like a one-time
    /// [System](crate::System).
    ///
    /// This is especially useful if you either need a lot of system data or,
    /// with Specs, if you want to build an entity and for that you need to
    /// access resources first - just fetching the resources and building
    /// the entity would cause a double borrow.
    ///
    /// **Calling this method is equivalent to:**
    ///
    /// ```
    /// # use shred::*;
    /// # struct MySystemData; impl MySystemData { fn do_something(&self) {} }
    /// # impl<'a> SystemData<'a> for MySystemData {
    /// #     fn fetch(res: &World) -> Self { MySystemData }
    /// #     fn reads() -> Vec<ResourceId> { vec![] }
    /// #     fn writes() -> Vec<ResourceId> { vec![] }
    /// #     fn setup(res: &mut World) {}
    /// # }
    /// # let mut world = World::empty();
    /// {
    ///     // note the extra scope
    ///     world.setup::<MySystemData>();
    ///     let my_data: MySystemData = world.system_data();
    ///     my_data.do_something();
    /// }
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use shred::*;
    /// // NOTE: If you use Specs, use `World::new` instead.
    /// let mut world = World::empty();
    ///
    /// #[derive(Default)]
    /// struct MyRes {
    ///     field: i32,
    /// }
    ///
    /// world.exec(|(mut my_res,): (Write<MyRes>,)| {
    ///     assert_eq!(my_res.field, 0);
    ///     my_res.field = 5;
    /// });
    ///
    /// assert_eq!(world.fetch::<MyRes>().field, 5);
    /// ```
    pub fn exec<'a, F, R, T>(&'a mut self, f: F) -> R
    where
        F: FnOnce(T) -> R,
        T: SystemData<'a>,
    {
        self.setup::<T>();
        f(self.system_data())
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
            inner: Ref::map(r.borrow(), Box::as_ref),
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
        id.assert_same_type_id::<T>();

        self.resources.get(&id).map(|r| Fetch {
            inner: Ref::map(r.borrow(), Box::as_ref),
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
            inner: RefMut::map(r.borrow_mut(), Box::as_mut),
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
        id.assert_same_type_id::<T>();

        self.resources.get(&id).map(|r| FetchMut {
            inner: RefMut::map(r.borrow_mut(), Box::as_mut),
            phantom: PhantomData,
        })
    }

    /// Internal function for inserting resources, should only be used if you
    /// know what you're doing.
    ///
    /// This is useful for inserting resources with a custom `ResourceId`.
    ///
    /// # Panics
    ///
    /// This method panics if `id` refers to a different type ID than `R`.
    pub fn insert_by_id<R>(&mut self, id: ResourceId, r: R)
    where
        R: Resource,
    {
        id.assert_same_type_id::<R>();

        self.resources.insert(id, TrustCell::new(Box::new(r)));
    }

    /// Internal function for removing resources, should only be used if you
    /// know what you're doing.
    ///
    /// This is useful for removing resources with a custom `ResourceId`.
    ///
    /// # Panics
    ///
    /// This method panics if `id` refers to a different type ID than `R`.
    pub fn remove_by_id<R>(&mut self, id: ResourceId) -> Option<R>
    where
        R: Resource,
    {
        // False-positive
        #![allow(clippy::redundant_closure)]

        id.assert_same_type_id::<R>();

        self.resources
            .remove(&id)
            .map(TrustCell::into_inner)
            .map(|x: Box<dyn Resource>| x.downcast())
            .map(|x: Result<Box<R>, _>| x.ok().unwrap())
            .map(|x| *x)
    }

    /// Internal function for fetching resources, should only be used if you
    /// know what you're doing.
    pub fn try_fetch_internal(&self, id: ResourceId) -> Option<&TrustCell<Box<dyn Resource>>> {
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
    pub fn get_mut_raw(&mut self, id: ResourceId) -> Option<&mut dyn Resource> {
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

        let mut world = World::empty();
        world.insert(Res);
        <Read<Res> as SystemData>::fetch(&world);
    }

    #[test]
    fn fetch_mut_aspects() {
        assert_eq!(Write::<Res>::reads(), vec![]);
        assert_eq!(Write::<Res>::writes(), vec![ResourceId::new::<Res>()]);

        let mut world = World::empty();
        world.insert(Res);
        <Write<Res> as SystemData>::fetch(&world);
    }

    #[test]
    fn fetch_by_id() {
        #![allow(clippy::map_clone)] // False positive

        let mut world = World::empty();

        world.insert_by_id(ResourceId::new_with_dynamic_id::<i32>(1), 5);
        world.insert_by_id(ResourceId::new_with_dynamic_id::<i32>(2), 15);
        world.insert_by_id(ResourceId::new_with_dynamic_id::<i32>(3), 45);

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
    fn system_data() {
        let mut world = World::empty();

        world.insert(5u32);
        let x = *world.system_data::<Read<u32>>();
        assert_eq!(x, 5);
    }

    #[test]
    fn setup() {
        let mut world = World::empty();

        world.insert(5u32);
        world.setup::<Read<u32>>();
        let x = *world.system_data::<Read<u32>>();
        assert_eq!(x, 5);

        world.remove::<u32>();
        world.setup::<Read<u32>>();
        let x = *world.system_data::<Read<u32>>();
        assert_eq!(x, 0);
    }

    #[test]
    fn exec() {
        #![allow(clippy::float_cmp)]

        let mut world = World::empty();

        world.exec(|(float, boolean): (Read<f32>, Read<bool>)| {
            assert_eq!(*float, 0.0);
            assert_eq!(*boolean, false);
        });

        world.exec(|(mut float, mut boolean): (Write<f32>, Write<bool>)| {
            *float = 4.3;
            *boolean = true;
        });

        world.exec(|(float, boolean): (Read<f32>, ReadExpect<bool>)| {
            assert_eq!(*float, 4.3);
            assert_eq!(*boolean, true);
        });
    }

    #[test]
    #[should_panic]
    fn exec_panic() {
        let mut world = World::empty();

        world.exec(|(_float, _boolean): (Write<f32>, Write<bool>)| {
            panic!();
        });
    }

    #[test]
    #[should_panic]
    fn invalid_fetch_by_id0() {
        let mut world = World::empty();

        world.insert(5i32);

        world.try_fetch_by_id::<u32>(ResourceId::new_with_dynamic_id::<i32>(111));
    }

    #[test]
    #[should_panic]
    fn invalid_fetch_by_id1() {
        let mut world = World::empty();

        world.insert(5i32);

        world.try_fetch_by_id::<i32>(ResourceId::new_with_dynamic_id::<u32>(111));
    }

    #[test]
    fn add() {
        struct Foo;

        let mut world = World::empty();
        world.insert(Res);

        assert!(world.has_value::<Res>());
        assert!(!world.has_value::<Foo>());
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "but it was already borrowed")]
    fn read_write_fails() {
        let mut world = World::empty();
        world.insert(Res);

        let read: Fetch<Res> = world.fetch();
        let write: FetchMut<Res> = world.fetch_mut();
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "but it was already borrowed mutably")]
    fn write_read_fails() {
        let mut world = World::empty();
        world.insert(Res);

        let write: FetchMut<Res> = world.fetch_mut();
        let read: Fetch<Res> = world.fetch();
    }

    #[test]
    fn remove_insert() {
        let mut world = World::empty();

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

        let mut world = World::empty();
        assert!(world.try_fetch::<i32>().is_none());

        let mut sys = Sys;
        RunNow::setup(&mut sys, &mut world);

        sys.run_now(&world);

        assert!(world.try_fetch::<i32>().is_some());
        assert_eq!(*world.fetch::<i32>(), 33);
    }
}
