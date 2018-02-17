//! Module for resource related types

pub use self::entry::Entry;

use std::any::TypeId;
#[cfg(feature = "nightly")]
use std::intrinsics::type_name;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use fxhash::FxHashMap;
use mopa::Any;
use parking_lot::Mutex;

use self::entry::create_entry;
use cell::{Ref, RefMut, TrustCell};
use system::SystemData;

mod entry;

#[cfg(feature = "nightly")]
macro_rules! fetch_panic {
    () => {
        {
            panic!(
                "Tried to fetch a resource of type {:?}, but the resource does not exist.\n\
                 Try adding the resource or \
                 using `Option<Fetch>` / `Fetch` instead of `FetchExpect`.",
                unsafe { type_name::<T>() },
            )
        }
    };
}

#[cfg(not(feature = "nightly"))]
macro_rules! fetch_panic {
    () => {
        {
            panic!(
                "Tried to fetch a resource, but the resource does not exist.\n\
                 Try adding the resource or \
                 using `Option<Fetch>` / `Fetch` instead of `FetchExpect`.\n\
                 You can get the type name of the resource by enabling `shred`'s `nightly` feature"
            )
        }
    };
}

/// Allows to fetch a resource in a system immutably.
///
/// This requires a `Default` implementation for the resource.
/// If the resource does not have a `Default` implementation or
/// it isn't strictly required, you should use `Option<Fetch<T>>`.
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

impl<'a, T> SystemData<'a> for Fetch<'a, T>
where
    T: Default + Resource,
{
    fn fetch(res: &'a Resources) -> Self {
        res.fetch()
    }

    fn reads() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }
}

/// Allows to fetch a resource in a system immutably.
/// **This will panic if the resource does not exist.**
/// Usage of `Fetch` or `Option<Fetch>` is therefore recommended.
///
/// If the `nightly` feature of `shred` is enabled, this will print
/// the type of the resource in case of a panic. That can help for debugging.
pub struct FetchExpect<'a, T: 'a> {
    f: Fetch<'a, T>,
}

impl<'a, T> Deref for FetchExpect<'a, T>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        &self.f
    }
}

impl<'a, T> SystemData<'a> for FetchExpect<'a, T>
where
    T: Resource,
{
    fn fetch(res: &'a Resources) -> Self {
        match res.try_fetch() {
            Some(f) => FetchExpect { f },
            None => fetch_panic!(),
        }
    }

    fn reads() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }
}

/// Allows to fetch a resource in a system mutably.
///
/// This requires a `Default` implementation for the resource.
/// If the resource does not have a `Default` implementation or
/// it isn't strictly required, you should use `Option<FetchMut<T>>`.
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

impl<'a, T> SystemData<'a> for FetchMut<'a, T>
where
    T: Default + Resource,
{
    fn fetch(res: &'a Resources) -> Self {
        res.fetch_mut()
    }

    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
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
pub struct FetchMutExpect<'a, T: 'a> {
    f: FetchMut<'a, T>,
}

impl<'a, T> Deref for FetchMutExpect<'a, T>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        &self.f
    }
}

impl<'a, T> DerefMut for FetchMutExpect<'a, T>
where
    T: Resource,
{
    fn deref_mut(&mut self) -> &mut T {
        &mut self.f
    }
}

impl<'a, T> SystemData<'a> for FetchMutExpect<'a, T>
where
    T: Resource,
{
    fn fetch(res: &'a Resources) -> Self {
        match res.try_fetch_mut() {
            Some(f) => FetchMutExpect { f },
            None => fetch_panic!(),
        }
    }

    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }
}

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

/// A resource container, which
/// provides methods to access to
/// the contained resources.
///
/// # Resource Ids
///
/// Resources are in general identified
/// by `ResourceId`, which consists of a `TypeId`.
#[derive(Default)]
pub struct Resources {
    new: Mutex<FxHashMap<ResourceId, Box<TrustCell<Box<Resource>>>>>,
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
    /// See `add_assert` for the panicking version.
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
    /// This was the old behavior of `add`.
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
    pub fn add_assert<R>(&mut self, r: R)
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

        self.maintain();

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

    /// Returns true if the specified resource type exists in `self`.
    /// This also checks resources that were added after the last `maintain`.
    /// If you only want to check maintained resources, use `has_maintained_value`.
    /// If you only want to add a resource if it hasn't been added yet, use `entry`.
    pub fn has_value(&self, id: ResourceId) -> bool {
        self.has_maintained_value(id) || self.new.lock().contains_key(&id)
    }

    /// Returns true if the specified resource type exists in `self`.
    /// Note that this only checks if there's a maintained resource of that id.
    pub fn has_maintained_value(&self, id: ResourceId) -> bool {
        self.resources.contains_key(&id)
    }

    /// Returns an entry for the resource with type `R` and id 0.
    /// This calls `maintain` before creating the `Entry`.
    pub fn entry<R>(&mut self) -> Entry<R>
    where
        R: Resource,
    {
        self.maintain();

        create_entry(self.resources.entry(ResourceId::new::<R>()))
    }

    /// Maintains the resources. This merges resources into the persistent
    /// map. When `fetch`ing a resource that is not available, a `Default` will
    /// be created. These values are inserted into a temporary map and get merged
    /// on each `maintain`.
    ///
    /// The following methods are checking the temporary map:
    ///
    /// * `has_value`
    /// * `try_fetch`
    /// * `try_fetch_mut`
    ///
    /// The following methods call `maintain` at the beginning:
    ///
    /// * `add`
    /// * `add_assert`
    /// * `add_or_overwrite`
    /// * `entry`
    ///
    /// These methods do not take care of unmaintained resources:
    ///
    /// * `has_maintained_value`
    pub fn maintain(&mut self) {
        self.resources
            .extend(self.new.get_mut().drain().map(|(k, b)| (k, *b)));
    }

    /// Fetches the resource with the specified type `T`.
    ///
    /// If the resource does not exist, the value returned by `T::default()` will
    /// be stored in a temporary map and fetched from there.
    /// See `maintain` for more information.
    ///
    /// # Panics
    ///
    /// Panics if the resource is being accessed mutably.
    pub fn fetch<T>(&self) -> Fetch<T>
    where
        T: Default + Resource,
    {
        let res_id = ResourceId::new::<T>();
        let r = self.resources
            .get(&res_id)
            .unwrap_or_else(|| self.def_res::<T>(res_id));

        Fetch {
            inner: r.borrow(),
            phantom: PhantomData,
        }
    }

    /// Like `fetch`, but returns an `Option` instead of inserting a default value
    /// in case the resource does not exist.
    pub fn try_fetch<T>(&self) -> Option<Fetch<T>>
    where
        T: Resource,
    {
        let res_id = ResourceId::new::<T>();

        self.resources
            .get(&res_id)
            .or_else(|| self.get_res::<T>(&res_id))
            .map(|r| Fetch {
                inner: r.borrow(),
                phantom: PhantomData,
            })
    }

    /// Fetches the resource with the specified type `T` mutably.
    ///
    /// Please see `fetch` for details.
    pub fn fetch_mut<T>(&self) -> FetchMut<T>
    where
        T: Default + Resource,
    {
        let res_id = ResourceId::new::<T>();
        let r = self.resources
            .get(&res_id)
            .unwrap_or_else(|| self.def_res::<T>(res_id));

        FetchMut {
            inner: r.borrow_mut(),
            phantom: PhantomData,
        }
    }

    /// Like `fetch_mut`, but returns an `Option` instead of inserting a default value
    /// in case the resource does not exist.
    pub fn try_fetch_mut<T>(&self) -> Option<FetchMut<T>>
    where
        T: Resource,
    {
        let res_id = ResourceId::new::<T>();

        self.resources
            .get(&res_id)
            .or_else(|| self.get_res::<T>(&res_id))
            .map(|r| FetchMut {
                inner: r.borrow_mut(),
                phantom: PhantomData,
            })
    }

    fn def_res<T>(&self, res_id: ResourceId) -> &TrustCell<Box<Resource>>
    where
        T: Default + Resource,
    {
        let mut new = self.new.lock();
        let b: &mut Box<_> = new.entry(res_id)
            .or_insert_with(|| Box::new(TrustCell::new(Box::new(T::default()) as Box<Resource>)));

        unsafe {
            // This is correct because the returned can only live until the mutable borrow
            // of `Resources`. This `Box` also lives at least until another mutable borrow,
            // because the only way to drop it is calling `maintain` (which borrows `Resources`
            // mutably).
            // The raw pointer of a `Box` stays stable and valid as long as the `Box` doesn't
            // get dropped.

            let t: &TrustCell<_> = b.as_ref();
            let raw = t as *const TrustCell<_>;

            &*raw
        }
    }

    fn get_res<T>(&self, res_id: &ResourceId) -> Option<&TrustCell<Box<Resource>>>
    where
        T: Resource,
    {
        let new = self.new.lock();
        let b: &Box<_> = match new.get(&res_id) {
            Some(x) => x,
            None => return None,
        };

        unsafe {
            // This is correct because the returned can only live until the mutable borrow
            // of `Resources`. This `Box` also lives at least until another mutable borrow,
            // because the only way to drop it is calling `maintain` (which borrows `Resources`
            // mutably).
            // The raw pointer of a `Box` stays stable and valid as long as the `Box` doesn't
            // get dropped.

            let t: &TrustCell<_> = b.as_ref();
            let raw = t as *const TrustCell<_>;

            Some(&*raw)
        }
    }
}

#[cfg(test)]
mod tests {
    use {RunNow, System};
    use super::*;

    #[derive(Default)]
    struct Res;

    #[test]
    fn fetch_aspects() {
        assert_eq!(Fetch::<Res>::reads(), vec![ResourceId::new::<Res>()]);
        assert_eq!(Fetch::<Res>::writes(), vec![]);

        let mut res = Resources::new();
        res.add_assert(Res);
        Fetch::<Res>::fetch(&res);
    }

    #[test]
    fn fetch_mut_aspects() {
        assert_eq!(FetchMut::<Res>::reads(), vec![]);
        assert_eq!(FetchMut::<Res>::writes(), vec![ResourceId::new::<Res>()]);

        let mut res = Resources::new();
        res.add_assert(Res);
        FetchMut::<Res>::fetch(&res);
    }

    #[test]
    fn add() {
        struct Foo;

        let mut res = Resources::new();
        res.add_assert(Res);

        assert!(res.has_value(ResourceId::new::<Res>()));
        assert!(!res.has_value(ResourceId::new::<Foo>()));
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed")]
    fn read_write_fails() {
        let mut res = Resources::new();
        res.add_assert(Res);

        let read = res.fetch::<Res>();
        let write = res.fetch_mut::<Res>();
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed mutably")]
    fn write_read_fails() {
        let mut res = Resources::new();
        res.add_assert(Res);

        let write = res.fetch_mut::<Res>();
        let read = res.fetch::<Res>();
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
        assert!(res.try_fetch::<i32>().is_none());

        let mut sys = Sys;
        sys.run_now(&res);

        assert!(res.try_fetch::<i32>().is_some());
        assert_eq!(*res.fetch::<i32>(), 33);

        res.maintain();

        assert_eq!(*res.fetch::<i32>(), 33);
    }
}
