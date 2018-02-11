//! Module for resource related types

pub use self::entry::Entry;

use std::any::TypeId;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use fxhash::FxHashMap;
use mopa::Any;

use self::entry::create_entry;
use cell::{Ref, RefMut, TrustCell};
use system::SystemData;

mod entry;

const RESOURCE_NOT_FOUND: &str = "No resource with the given id";

/// Return value of [`Resources::fetch`].
///
/// [`Resources::fetch`]: struct.Resources.html#method.fetch
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
    T: Resource,
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

/// Return value of [`Resources::fetch_id`].
///
/// [`Resources::fetch_id`]: struct.Resources.html#method.fetch_id
pub struct FetchId<'a> {
    inner: Ref<'a, Box<Resource>>,
}

impl<'a> Deref for FetchId<'a> {
    type Target = Resource;

    fn deref(&self) -> &Resource {
        self.inner.as_ref()
    }
}

/// Return value of [`Resources::fetch_id_mut`].
///
/// [`Resources::fetch_id_mut`]: struct.Resources.html#method.fetch_id_mut
pub struct FetchIdMut<'a> {
    inner: RefMut<'a, Box<Resource>>,
}

impl<'a> Deref for FetchIdMut<'a> {
    type Target = Resource;

    fn deref(&self) -> &Resource {
        self.inner.as_ref()
    }
}

impl<'a> DerefMut for FetchIdMut<'a> {
    fn deref_mut(&mut self) -> &mut Resource {
        self.inner.as_mut()
    }
}

/// Return value of [`Resources::fetch_mut`].
///
/// [`Resources::fetch_mut`]: struct.Resources.html#method.fetch_mut
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
    T: Resource,
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
    resources: FxHashMap<ResourceId, TrustCell<Box<Resource>>>,
}

impl Resources {
    /// Creates a new, empty resource container.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a new resource to this container.
    ///
    /// # Panics
    ///
    /// Panics if the resource is already registered.
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
    pub fn add<R>(&mut self, r: R)
    where
        R: Resource,
    {
        use std::collections::hash_map::Entry;

        let entry = self.resources.entry(ResourceId::new::<R>());

        if let Entry::Vacant(e) = entry {
            e.insert(TrustCell::new(Box::new(r)));
        } else {
            panic!("Tried to add a resource though \
                    an instance of this type already exists in `Resources`");
        }
    }

    /// Returns true if the specified resource type exists in `self`.
    pub fn has_value(&self, id: ResourceId) -> bool {
        self.resources.contains_key(&id)
    }

    /// Returns an entry for the resource with type `R` and id 0.
    pub fn entry<R>(&mut self) -> Entry<R>
    where
        R: Resource,
    {
        create_entry(self.resources.entry(ResourceId::new::<R>()))
    }

    /// Fetches the resource with the specified type `T`.
    /// The id is useful if you don't define your resources
    /// in Rust or you want a more dynamic resource handling.
    /// By default, the `#[derive(SystemData)]` passes `()`
    /// as id.
    ///
    /// # Panics
    ///
    /// Panics if the resource is being accessed mutably.
    /// Also panics if there is no such resource.
    pub fn fetch<T>(&self) -> Fetch<T>
    where
        T: Resource,
    {
        self.try_fetch().expect(RESOURCE_NOT_FOUND)
    }

    /// Like `fetch`, but returns an `Option` instead of panicking in the case of the resource
    /// being accessed mutably.
    pub fn try_fetch<T>(&self) -> Option<Fetch<T>>
    where
        T: Resource,
    {
        self.try_fetch_internal(TypeId::of::<T>()).map(|r| {
            Fetch {
                inner: r.borrow(),
                phantom: PhantomData,
            }
        })
    }

    /// Fetches the resource with the specified type `T` mutably.
    ///
    /// Please see `fetch` for details.
    pub fn fetch_mut<T>(&self) -> FetchMut<T>
    where
        T: Resource,
    {
        self.try_fetch_mut().expect(RESOURCE_NOT_FOUND)
    }

    /// Like `fetch_mut`, but returns an `Option` instead of panicking in the case of the resource
    /// being accessed mutably.
    pub fn try_fetch_mut<T>(&self) -> Option<FetchMut<T>>
    where
        T: Resource,
    {
        self.try_fetch_internal(TypeId::of::<T>()).map(|r| {
            FetchMut {
                inner: r.borrow_mut(),
                phantom: PhantomData,
            }
        })
    }

    fn try_fetch_internal(&self, id: TypeId) -> Option<&TrustCell<Box<Resource>>> {
        self.resources.get(&ResourceId(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Res;

    #[test]
    fn fetch_aspects() {
        assert_eq!(
            Fetch::<Res>::reads(),
            vec![ResourceId::new::<Res>()]
        );
        assert_eq!(Fetch::<Res>::writes(), vec![]);

        let mut res = Resources::new();
        res.add(Res);
        Fetch::<Res>::fetch(&res);
    }

    #[test]
    fn fetch_mut_aspects() {
        assert_eq!(FetchMut::<Res>::reads(), vec![]);
        assert_eq!(
            FetchMut::<Res>::writes(),
            vec![ResourceId::new::<Res>()]
        );

        let mut res = Resources::new();
        res.add(Res);
        FetchMut::<Res>::fetch(&res);
    }

    #[test]
    fn add() {
        struct Foo;

        let mut res = Resources::new();
        res.add(Res);

        assert!(res.has_value(ResourceId::new::<Res>()));
        assert!(!res.has_value(ResourceId::new::<Foo>()));
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed")]
    fn read_write_fails() {
        let mut res = Resources::new();
        res.add(Res);

        let read = res.fetch::<Res>();
        let write = res.fetch_mut::<Res>();
    }

    #[allow(unused)]
    #[test]
    #[should_panic(expected = "Already borrowed mutably")]
    fn write_read_fails() {
        let mut res = Resources::new();
        res.add(Res);

        let write = res.fetch_mut::<Res>();
        let read = res.fetch::<Res>();
    }
}
