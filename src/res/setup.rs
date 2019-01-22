use {Resource, World};

#[cfg(feature = "nightly")]
macro_rules! fetch_panic {
    () => {{
        panic!(
            "Tried to fetch a resource of type {:?}, but the resource does not exist.\n\
             Try adding the resource by inserting it manually or using the `setup` method.",
            unsafe { ::std::intrinsics::type_name::<T>() },
        )
    }};
}

#[cfg(not(feature = "nightly"))]
macro_rules! fetch_panic {
    () => {{
        panic!(
            "Tried to fetch a resource, but the resource does not exist.\n\
             Try adding the resource by inserting it manually or using the `setup` method.\n\
             You can get the type name of the missing resource by enabling `shred`'s `nightly` \
             feature"
        )
    }};
}

/// A `SetupHandler` that simply uses the default implementation.
pub struct DefaultProvider;

impl<T> SetupHandler<T> for DefaultProvider
where
    T: Default + Resource,
{
    fn setup(res: &mut World) {
        res.entry().or_insert_with(T::default);
    }
}

/// A setup handler performing the fetching of `T`.
pub trait SetupHandler<T>: Sized {
    /// Sets up `World` for fetching `T`.
    fn setup(res: &mut World);
}

/// A setup handler that simply does nothing and thus will cause a panic on fetching.
/// The panic will provide the type name if the `nightly` feature of shred is enabled.
///
/// A typedef called `ReadExpect` exists, so you usually don't use this type directly.
pub struct PanicHandler;

impl<T> SetupHandler<T> for PanicHandler
where
    T: Resource,
{
    fn setup(_: &mut World) {}
}
