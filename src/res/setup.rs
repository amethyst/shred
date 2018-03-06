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

/// A `SetupHandler` that simply uses the default implementation.
pub struct DefaultProvider;

impl<T> SetupHandler<T> for DefaultProvider
where
    T: Default + Resource,
{
    fn setup(res: &mut Resources) {
        res.entry().or_insert_with(T::default);
    }
}

/// A setup handler performing the fetching of `T`.
pub trait SetupHandler<T>: Sized {
    /// Sets up `Resources` for fetching `T`.
    fn setup(res: &mut Resources);
}

/// A setup handler that panics if the resource does not exist.
/// This will provide the type name if the `nightly` feature of shred is enabled.
///
/// A typedef called `FetchExpect` exists, so you usually don't use this type directly.
pub struct PanicHandler;

impl<T> SetupHandler<T> for PanicHandler
where
    T: Resource,
{
    fn setup(res: &mut Resources) {
        if !res.has_value::<T>() {
            fetch_panic!()
        }
    }
}
