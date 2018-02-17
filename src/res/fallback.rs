#[cfg(feature = "nightly")]
use std::intrinsics::type_name;

use {Fetch, FetchMut, Resource, ResourceId, Resources};

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

/// A `FallbackHandler` that simply uses the default implementation.
pub struct DefaultProvider;

impl<T> FallbackHandler<T> for DefaultProvider
where
    T: Default + Resource,
{
    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }

    fn fetch(res: &Resources) -> Fetch<T, Self> {
        res.fetch(Default::default)
    }

    fn fetch_mut(res: &Resources) -> FetchMut<T, Self> {
        res.fetch_mut(Default::default)
    }
}

/// A fallback handler performing the fetching of `T`.
/// The implementor is responsible for providing a fallback in case the resource
/// does not exist.
pub trait FallbackHandler<T>: Sized {
    /// Returns the resource ids of all resources
    /// that get fetched immutably in `fallback`/`fallback_mut`.
    ///
    /// Do not include `T` here!
    fn reads() -> Vec<ResourceId>;

    /// Returns the resource ids of all resources
    /// that get fetched mutably in `fallback`/`fallback_mut`.
    ///
    /// Do not include `T` here!
    fn writes() -> Vec<ResourceId>;

    /// Called when fetching `Fetch<T>`.
    fn fetch(res: &Resources) -> Fetch<T, Self>;
    /// Called when fetching `FetchMut<T>`.
    fn fetch_mut(res: &Resources) -> FetchMut<T, Self>;
}

/// A fallback handler that panics if the resource does not exist.
/// This will provide the type name if the `nightly` feature of shred is enabled.
///
/// A typedef called `FetchExpect` exists, so you usually don't use this type directly.
pub struct PanicHandler;

impl<T> FallbackHandler<T> for PanicHandler
where
    T: Resource,
{
    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }

    fn fetch(res: &Resources) -> Fetch<T, Self> {
        match res.try_fetch() {
            Some(f) => f,
            None => fetch_panic!(),
        }
    }

    fn fetch_mut(res: &Resources) -> FetchMut<T, Self> {
        match res.try_fetch_mut() {
            Some(f) => f,
            None => fetch_panic!(),
        }
    }
}
