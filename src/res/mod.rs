//! Module for resource related types

pub use self::entry::Entry;
pub use self::setup::{DefaultProvider, SetupHandler, PanicHandler};

use std::any::TypeId;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use fxhash::FxHashMap;
use mopa::Any;
use parking_lot::Mutex;

use self::entry::create_entry;
use cell::{Ref, RefMut, TrustCell};
use system::SystemData;

mod entry;
mod setup;

/// Allows to fetch a resource in a system immutably.
///
/// This requires a `Default` implementation for the resource.
/// If the resource does not have a `Default` implementation or
/// it isn't strictly required, you should use `Option<Fetch<T>>`
/// or a custom fallback handler.
///
/// # Type parameters
///
/// * `T`: The type of the resource
/// * `F`: The fallback handler (default: `DefaultProvider`)
pub struct Fetch<'a, T: 'a, F = DefaultProvider> {
    inner: Ref<'a, Box<Resource>>,
    phantom: PhantomData<(&'a T, F)>,
}

impl<'a, T, F> Deref for Fetch<'a, T, F>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.inner.downcast_ref_unchecked() }
    }
}

impl<'a, T, F> SystemData<'a> for Fetch<'a, T, F>
where
    T: Resource,
    F: SetupHandler<T>,
{
    fn fetch(res: &'a Resources) -> Self {
        F::fetch(res)
    }

    fn reads() -> Vec<ResourceId> {
        let mut reads = F::reads();

        reads.push(ResourceId::new::<T>());

        reads
    }

    fn writes() -> Vec<ResourceId> {
        F::writes()
    }
}

/// Allows to fetch a resource in a system immutably.
/// **This will panic if the resource does not exist.**
/// Usage of `Fetch` or `Option<Fetch>` is therefore recommended.
///
/// If the `nightly` feature of `shred` is enabled, this will print
/// the type of the resource in case of a panic. That can help for debugging.
pub type FetchExpect<'a, T> = Fetch<'a, T, PanicHandler>;

/// Allows to fetch a resource in a system mutably.
///
/// This requires a `Default` implementation for the resource.
/// If the resource does not have a `Default` implementation or
/// it isn't strictly required, you should use `Option<FetchMut<T>>`
/// or a custom fallback handler.
///
/// # Type parameters
///
/// * `T`: The type of the resource
/// * `F`: The fallback handler (default: `DefaultProvider`)
pub struct FetchMut<'a, T: 'a, F = DefaultProvider> {
    inner: RefMut<'a, Box<Resource>>,
    phantom: PhantomData<(&'a mut T, F)>,
}

impl<'a, T, F> Deref for FetchMut<'a, T, F>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.inner.downcast_ref_unchecked() }
    }
}

impl<'a, T, F> DerefMut for FetchMut<'a, T, F>
where
    T: Resource,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.inner.downcast_mut_unchecked() }
    }
}

impl<'a, T, F> SystemData<'a> for FetchMut<'a, T, F>
where
    T: Default + Resource,
    F: SetupHandler<T>,
{
    fn fetch(res: &'a Resources) -> Self {
        F::fetch_mut(res)
    }

    fn reads() -> Vec<ResourceId> {
        F::reads()
    }

    fn writes() -> Vec<ResourceId> {
        let mut writes = F::writes();

        writes.push(ResourceId::new::<T>());

        writes
    }
}

impl<'a, T> SystemData<'a> for Option<Fetch<'a, T>>
where
    T: Resource,
{
    fn fetch(res: &'a Resources) -> Self {
        res.try_fetch()
    }

    fn reads() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }
}

impl<'a, T> SystemData<'a> for Option<FetchMut<'a, T>>
where
    T: Resource,
{
    fn fetch(res: &'a Resources) -> Self {
        res.try_fetch_mut()
    }

    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }
}

/// Allows to fetch a resource in a system mutably.
/// **This will panic if the resource does not exist.**
/// Usage of `FetchMut` or `Option<FetchMut>` is therefore recommended.
///
/// If the `nightly` feature of `shred` is enabled, this will print
/// the type of the resource in case of a panic. That can help for debugging.
pub type FetchMutExpect<'a, T> = FetchMut<'a, T, PanicHandler>;

/// A resource defines a set of data
/// which can only be accessed according
/// to Rust's typical borrowing model (one writer xor multiple readers).
pub trait Resource: Any + Send + Sync {}

mopafy!(Resource);

impl<T> Resource for T
where
    T: Any + Send + Sync,
{
}

/// The id of a [`Resource`],
/// which is a tuple struct with a type
/// id and an additional resource id (represented with a `usize`).
///
/// The default resource id is `0`.
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
pub struct Resources {
    resources: FxHashMap<ResourceId, TrustCell<Box<Resource>>>,
}

impl Resources {
    /// Creates a new, empty resource container.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a new resource to this container, or returns `r` again if the resource exists already.
    /// Calls `maintain` at the beginning, so a resource added by `fetch` or `fetch_mut`
    /// will also cause `Some(r)` to be returned.
    ///
    /// Returns `None` if the resource has been added successfully.
    ///
    /// # Other variants
    ///
    /// See `add_no_overwrite` for the panicking version.
    /// See `add_or_overwrite` for a version that just assures a resource has a certain value.
    ///
    /// # Examples
    ///
    /// Every type satisfying `Any + Debug + Send + Sync`
    /// automatically implements `Resource`:
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// #[derive(Debug)]
    /// struct MyRes(i32);
    /// ```
    ///
    /// When you have a resource, simply
    /// register it like this:
    ///
    /// ```rust
    /// # #[derive(Debug)] struct MyRes(i32);
    /// use shred::Resources;
    ///
    /// let mut res = Resources::new();
    /// res.add(MyRes(5));
    /// ```
    pub fn add<R>(&mut self, r: R) -> Option<R>
    where
        R: Resource,
    {
        use std::collections::hash_map::Entry;

        self.maintain();

        let entry = self.resources.entry(ResourceId::new::<R>());

        if let Entry::Vacant(e) = entry {
            e.insert(TrustCell::new(Box::new(r)));

            None
        } else {
            Some(r)
        }
    }

    /// Adds a resource or panics if it already exists.
    ///
    /// Calls `maintain` at the beginning.
    ///
    /// # Panics
    ///
    /// Panics if the resource has already been added.
    ///
    /// # Other variants
    ///
    /// See `add` for the non-panicking version.
    /// See `add_or_overwrite` for a version that just assures a resource has a certain value.
    pub fn add_no_overwrite<R>(&mut self, r: R)
    where
        R: Resource,
    {
        if self.add(r).is_some() {
            panic!(
                "Tried to add a resource though an instance \
                 of this type already exists in `Resources`"
            );
        }
    }

    /// Adds a new resource to this container or changes the existing resource of type `R`.
    /// Calls `maintain` at the beginning.
    ///
    /// # Examples
    ///
    /// Every type satisfying `Any + Debug + Send + Sync`
    /// automatically implements `Resource`:
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// #[derive(Debug)]
    /// struct MyRes(i32);
    /// ```
    ///
    /// When you have a resource, simply
    /// add it like this:
    ///
    /// ```rust
    /// # #[derive(Debug)] struct MyRes(i32);
    /// use shred::Resources;
    ///
    /// let mut res = Resources::new();
    /// res.add_or_overwrite(MyRes(5));
    /// // ...
    /// res.add_or_overwrite(MyRes(4)); // `MyRes` has now a value of 4
    /// ```
    pub fn add_or_overwrite<R>(&mut self, r: R)
    where
        R: Resource,
    {
        use std::collections::hash_map::Entry;

        let entry = self.resources.entry(ResourceId::new::<R>());

        match entry {
            Entry::Occupied(mut occ) => unsafe {
                *occ.get_mut().get_mut().downcast_mut_unchecked() = r;
            },
            Entry::Vacant(vac) => {
                vac.insert(TrustCell::new(Box::new(r)));
            }
        }
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

    /// Fetches the resource with the specified type `T` or panics if it doesn't exist.
    ///
    /// # Panics
    ///
    /// Panics if the resource doesn't exist.
    /// Panics if the resource is being accessed mutably.
    pub fn fetch<T, P>(&self) -> Fetch<T, P>
    where
        T: Resource,
    {
        let res_id = ResourceId::new::<T>();
        let r = self.resources
            .get(&res_id)
            .unwrap_or_else(|| self.def_res(res_id, with()));

        Fetch {
            inner: r.borrow(),
            phantom: PhantomData,
        }
    }

    /// Like `fetch`, but returns an `Option` instead of inserting a default value
    /// in case the resource does not exist.
    pub fn try_fetch<T, P>(&self) -> Option<Fetch<T, P>>
    where
        T: Resource,
    {
        let res_id = ResourceId::new::<T>();

        self.resources
            .get(&res_id)
            .map(|r| Fetch {
                inner: r.borrow(),
                phantom: PhantomData,
            })
    }

    /// Fetches the resource with the specified type `T` mutably.
    ///
    /// Please see `fetch` for details.
    pub fn fetch_mut<T, F, P>(&self, with: F) -> FetchMut<T, P>
    where
        T: Resource,
        F: FnOnce() -> T,
    {
        let res_id = ResourceId::new::<T>();
        let r = self.resources
            .get(&res_id)
            .unwrap_or_else(|| self.def_res::<T>(res_id, with()));

        FetchMut {
            inner: r.borrow_mut(),
            phantom: PhantomData,
        }
    }

    /// Like `fetch_mut`, but returns an `Option` instead of inserting a default value
    /// in case the resource does not exist.
    pub fn try_fetch_mut<T, P>(&self) -> Option<FetchMut<T, P>>
    where
        T: Resource,
    {
        let res_id = ResourceId::new::<T>();

        self.resources
            .get(&res_id)
            .map(|r| FetchMut {
                inner: r.borrow_mut(),
                phantom: PhantomData,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use {RunNow, System};

    #[derive(Default)]
    struct Res;

    #[test]
    fn fetch_aspects() {
        assert_eq!(Fetch::<Res>::reads(), vec![ResourceId::new::<Res>()]);
        assert_eq!(Fetch::<Res>::writes(), vec![]);

        let mut res = Resources::new();
        res.add_no_overwrite(Res);
        Fetch::<Res>::fetch(&res);
    }

    #[test]
    fn fetch_mut_aspects() {
        assert_eq!(FetchMut::<Res>::reads(), vec![]);
        assert_eq!(FetchMut::<Res>::writes(), vec![ResourceId::new::<Res>()]);

        let mut res = Resources::new();
        res.add_no_overwrite(Res);
        FetchMut::<Res>::fetch(&res);
    }

    #[test]
    fn add() {
        struct Foo;

        let mut res = Resources::new();
        res.add_no_overwrite(Res);

        assert!(res.has_value::<Res>());
        assert!(!res.has_value::<Foo>());
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed")]
    fn read_write_fails() {
        let mut res = Resources::new();
        res.add_no_overwrite(Res);

        let read: Fetch<Res> = res.fetch(Default::default);
        let write: FetchMut<Res> = res.fetch_mut(Default::default);
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed mutably")]
    fn write_read_fails() {
        let mut res = Resources::new();
        res.add_no_overwrite(Res);

        let write: FetchMut<Res> = res.fetch_mut(Default::default);
        let read: Fetch<Res> = res.fetch(Default::default);
    }

    #[test]
    fn default_works() {
        struct Sys;

        impl<'a> System<'a> for Sys {
            type SystemData = FetchMut<'a, i32>;

            fn run(&mut self, mut data: Self::SystemData) {
                assert_eq!(*data, 0);

                *data = 33;
            }
        }

        let mut res = Resources::new();
        assert!(res.try_fetch::<i32, ()>().is_none());

        let mut sys = Sys;
        sys.run_now(&res);

        assert!(res.try_fetch::<i32, ()>().is_some());
        assert_eq!(*res.fetch::<i32, _, ()>(Default::default), 33);

        res.maintain();

        assert_eq!(*res.fetch::<i32, _, ()>(Default::default), 33);
    }
}
