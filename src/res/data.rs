use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use {DefaultProvider, Fetch, FetchMut, PanicHandler, Resource, ResourceId, Resources,
     SetupHandler, SystemData};

/// Allows to fetch a resource in a system immutably.
///
/// If the resource isn't strictly required, you should use `Option<Fetch<T>>`.
///
/// # Type parameters
///
/// * `T`: The type of the resource
/// * `F`: The setup handler (default: `DefaultProvider`)
pub struct Read<'a, T: 'a, F = DefaultProvider> {
    inner: Fetch<'a, T>,
    phantom: PhantomData<F>,
}

impl<'a, T, F> Deref for Read<'a, T, F>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        &*self.inner
    }
}

impl<'a, T, F> From<Fetch<'a, T>> for Read<'a, T, F> {
    fn from(inner: Fetch<'a, T>) -> Self {
        Read {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<'a, T, F> SystemData<'a> for Read<'a, T, F>
where
    T: Resource,
    F: SetupHandler<T>,
{
    fn setup(res: &mut Resources) {
        F::setup(res)
    }

    fn fetch(res: &'a Resources) -> Self {
        res.fetch::<T>().into()
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
pub struct Write<'a, T: 'a, F = DefaultProvider> {
    inner: FetchMut<'a, T>,
    phantom: PhantomData<F>,
}

impl<'a, T, F> Deref for Write<'a, T, F>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &T {
        &*self.inner
    }
}

impl<'a, T, F> DerefMut for Write<'a, T, F>
where
    T: Resource,
{
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.inner
    }
}

impl<'a, T, F> From<FetchMut<'a, T>> for Write<'a, T, F> {
    fn from(inner: FetchMut<'a, T>) -> Self {
        Write {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<'a, T, F> SystemData<'a> for Write<'a, T, F>
where
    T: Resource,
    F: SetupHandler<T>,
{
    fn setup(res: &mut Resources) {
        F::setup(res)
    }

    fn fetch(res: &'a Resources) -> Self {
        res.fetch_mut::<T>().into()
    }

    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }
}

// ------------------

impl<'a, T, F> SystemData<'a> for Option<Read<'a, T, F>>
where
    T: Resource,
{
    fn setup(_: &mut Resources) {}

    fn fetch(res: &'a Resources) -> Self {
        res.try_fetch().map(Into::into)
    }

    fn reads() -> Vec<ResourceId> {
        vec![ResourceId::new::<T>()]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }
}

impl<'a, T, F> SystemData<'a> for Option<Write<'a, T, F>>
where
    T: Resource,
{
    fn setup(_: &mut Resources) {}

    fn fetch(res: &'a Resources) -> Self {
        res.try_fetch_mut().map(Into::into)
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
pub type ReadExpect<'a, T> = Read<'a, T, PanicHandler>;

/// Allows to fetch a resource in a system mutably.
/// **This will panic if the resource does not exist.**
/// Usage of `FetchMut` or `Option<FetchMut>` is therefore recommended.
///
/// If the `nightly` feature of `shred` is enabled, this will print
/// the type of the resource in case of a panic. That can help for debugging.
pub type WriteExpect<'a, T> = Write<'a, T, PanicHandler>;
