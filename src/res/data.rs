use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use {DefaultProvider, Fetch, FetchMut, PanicHandler, Resource, ResourceId, Resources, SetupHandler,
     SystemData};
use cell::{Ref, RefMut};
use system::SystemFetch;

/// Allows to fetch a resource in a system immutably.
///
/// If the resource isn't strictly required, you should use `Option<Read<T>>`.
///
/// # Type parameters
///
/// * `T`: The type of the resource
/// * `F`: The setup handler (default: `DefaultProvider`)
pub struct Read<T, F = DefaultProvider> {
    inner: Ref<Box<Resource>>,
    p1: PhantomData<T>,
    p2: PhantomData<F>,
}

impl<T, F> Deref for Read<T, F>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.inner.downcast_ref_unchecked() }
    }
}

impl<'a, T, F> From<Fetch<'a, T>> for Read<T, F> {
    fn from(inner: Fetch<'a, T>) -> Self {
        Read {
            inner: inner.into_inner(),
            p1: PhantomData,
            p2: PhantomData,
        }
    }
}

impl<T, F> SystemData for Read<T, F>
where
    T: Resource,
    F: SetupHandler<T>,
{
    fn setup(res: &mut Resources) {
        F::setup(res)
    }

    fn fetch<'r>(res: &'r Resources) -> SystemFetch<'r, Self> {
        SystemFetch::from(Read::from(res.fetch::<T>()))
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
/// If the resource isn't strictly required, you should use `Option<Write<T>>`.
///
/// # Type parameters
///
/// * `T`: The type of the resource
/// * `F`: The setup handler (default: `DefaultProvider`)
pub struct Write<T, F = DefaultProvider> {
    inner: RefMut<Box<Resource>>,
    p1: PhantomData<T>,
    p2: PhantomData<F>,
}

impl<T, F> Deref for Write<T, F>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.inner.downcast_ref_unchecked() }
    }
}

impl<T, F> DerefMut for Write<T, F>
where
    T: Resource,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.downcast_mut_unchecked() }
    }
}

impl<'a, T, F> From<FetchMut<'a, T>> for Write<T, F> {
    fn from(inner: FetchMut<'a, T>) -> Self {
        Write {
            inner: inner.into_inner(),
            p1: PhantomData,
            p2: PhantomData,
        }
    }
}

impl<T, F> SystemData for Write<T, F>
where
    T: Resource,
    F: SetupHandler<T>,
{
    fn setup(res: &mut Resources) {
        F::setup(res)
    }

    fn fetch<'r>(res: &'r Resources) -> SystemFetch<'r, Self> {
        SystemFetch::from(Write::from(res.fetch_mut::<T>()))
    }

    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }
}

// ------------------

impl<T, F> SystemData for Option<Read<T, F>>
where
    T: Resource,
{
    fn setup(_: &mut Resources) {}

    fn fetch<'r>(res: &'r Resources) -> SystemFetch<'r, Self> {
        SystemFetch::from(res.try_fetch().map(Read::from))
    }

    fn reads() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }
}

impl<T, F> SystemData for Option<Write<T, F>>
where
    T: Resource,
{
    fn setup(_: &mut Resources) {}

    fn fetch<'r>(res: &'r Resources) -> SystemFetch<'r, Self> {
        SystemFetch::from(res.try_fetch_mut().map(Write::from))
    }

    fn reads() -> Vec<ResourceId> {
        vec![]
    }
    fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }
}

/// Allows to fetch a resource in a system immutably.
/// **This will panic if the resource does not exist.**
/// Usage of `Read` or `Option<Read>` is therefore recommended.
///
/// If the `nightly` feature of `shred` is enabled, this will print
/// the type of the resource in case of a panic. That can help for debugging.
pub type ReadExpect<T> = Read<T, PanicHandler>;

/// Allows to fetch a resource in a system mutably.
/// **This will panic if the resource does not exist.**
/// Usage of `Write` or `Option<Write>` is therefore recommended.
///
/// If the `nightly` feature of `shred` is enabled, this will print
/// the type of the resource in case of a panic. That can help for debugging.
pub type WriteExpect<T> = Write<T, PanicHandler>;
