//! Module for resource related types

use std::any::TypeId;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use fnv::{FnvHasher, FnvHashMap};
use mopa::Any;

use cell::{Ref, RefMut, TrustCell};

/// Return value of [`Resources::fetch`].
///
/// [`Resources::fetch`]: struct.Resources.html#method.fetch
pub struct Fetch<'a, T: 'a> {
    inner: Ref<'a, Box<Resource>>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> Deref for Fetch<'a, T>
    where T: Resource
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.inner.downcast_ref_unchecked() }
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
    where T: Resource
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.inner.downcast_ref_unchecked() }
    }
}

impl<'a, T> DerefMut for FetchMut<'a, T>
    where T: Resource
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.inner.downcast_mut_unchecked() }
    }
}

/// A resource defines a set of data
/// which can only be accessed according
/// to Rust's typical borrowing model (one writer xor multiple readers).
pub trait Resource: Any + Send + Sync {}

mopafy!(Resource);

/// The id of a [`Resource`],
/// which is the same as it's type
/// id at the moment.
///
/// [`Resource`]: trait.Resource.html
pub type ResourceId = (TypeId, u64);

/// A resource container, which
/// provides methods to access to
/// the contained resources.
pub struct Resources {
    resources: FnvHashMap<ResourceId, TrustCell<Box<Resource>>>,
}

impl Resources {
    /// Creates a new, empty resource container.
    pub fn new() -> Self {
        Resources { resources: Default::default() }
    }

    /// Adds a new resource
    /// to this container.
    ///
    /// # Panics
    ///
    /// Panics if the resource is already registered.
    ///
    /// # Examples
    ///
    /// To make a type a resource, simply
    /// implement the [`Resource`] trait:
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// #
    /// use shred::Resource;
    ///
    /// struct MyRes(i32);
    ///
    /// impl Resource for MyRes {}
    /// ```
    ///
    /// When you have a resource, simply
    /// register it like this:
    ///
    /// ```rust
    /// # use shred::Resource;
    /// #
    /// # struct MyRes(i32);
    /// #
    /// # impl Resource for MyRes {}
    /// use shred::Resources;
    ///
    /// let mut res = Resources::new();
    /// res.add(MyRes(5), 0);
    /// ```
    pub fn add<R, ID>(&mut self, r: R, id: ID)
        where R: Resource,
              ID: Hash + Eq
    {
        use std::collections::hash_map::Entry;
        use std::hash::Hasher;

        let id = fnv_hash(id);
        let entry = self.resources.entry((TypeId::of::<R>(), id));

        if let Entry::Vacant(e) = entry {
            e.insert(TrustCell::new(Box::new(r)));
        } else {
            panic!("Tried to add a resource though it is already registered");
        }
    }

    /// Fetches the resource with the specified type `T`.
    ///
    /// # Safety
    ///
    /// This method is marked as `unsafe` as calling
    /// it means you are sure about the implied borrowing
    /// rules.
    ///
    /// # Panics
    ///
    /// Panics if the resource is being accessed mutably.
    /// Also panics if there is no such resource.
    pub unsafe fn fetch<T, ID>(&self, id: ID) -> Fetch<T>
        where T: Resource,
              ID: Hash + Eq
    {
        let c = self.fetch_internal(TypeId::of::<T>(), id);

        Fetch {
            inner: c.borrow(),
            phantom: PhantomData,
        }
    }

    /// Fetches the resource with the specified type `T` mutably.
    ///
    /// Please see `fetch` for details.
    pub unsafe fn fetch_mut<T, ID>(&self, id: ID) -> FetchMut<T>
        where T: Resource,
              ID: Hash + Eq
    {
        let c = self.fetch_internal(TypeId::of::<T>(), id);

        FetchMut {
            inner: c.borrow_mut(),
            phantom: PhantomData,
        }
    }

    /// Fetches the resource with the specified type id.
    ///
    /// Please see `fetch` for details.
    pub unsafe fn fetch_id<ID>(&self, id: TypeId, comp_id: ID) -> FetchId
        where ID: Hash + Eq
    {
        let c = self.fetch_internal(id, comp_id);

        FetchId { inner: c.borrow() }
    }

    /// Fetches the resource with the specified type id mutably.
    ///
    /// Please see `fetch` for details.
    pub unsafe fn fetch_id_mut<ID>(&self, id: TypeId, comp_id: ID) -> FetchIdMut
        where ID: Hash + Eq
    {
        let c = self.fetch_internal(id, comp_id);

        FetchIdMut { inner: c.borrow_mut() }
    }

    fn fetch_internal<ID>(&self, id: TypeId, cid: ID) -> &TrustCell<Box<Resource>>
        where ID: Hash + Eq
    {
        self.resources
            .get(&(id, fnv_hash(cid)))
            .expect("No resource with the given id")
    }
}

fn fnv_hash<H: Hash>(h: H) -> u64 {
    use std::hash::Hasher;

    let mut hasher = FnvHasher::default();
    Hash::hash(&h, &mut hasher);
    hasher.finish()
}
