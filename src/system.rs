use std::marker::PhantomData;
use std::ops::Deref;

use {ResourceId, Resources};

/// A trait for accessing read/write multiple resources from a system. This can be used
/// to create dynamic systems that don't specify what they fetch at compile-time.
///
/// For compile-time system data this will all be done for you using `StaticAccessor`.
pub trait Accessor: Sized {
    /// Tries to create a new instance of this type. This one returns `Some` in case there is a
    /// default, otherwise the system needs to override `System::accessor`.
    fn try_new() -> Option<Self>;

    /// A list of [`ResourceId`]s the bundle
    /// needs read access to in order to
    /// build the target resource bundle.
    ///
    /// # Contract
    ///
    /// Exactly return the dependencies you're going to `fetch`! Doing otherwise *will* cause a
    /// panic.
    ///
    /// This method is only executed once,
    /// thus the returned value may never change
    /// (otherwise it has no effect).
    ///
    /// [`ResourceId`]: struct.ResourceId.html
    fn reads(&self) -> Vec<ResourceId>;

    /// A list of [`ResourceId`]s the bundle
    /// needs write access to in order to
    /// build the target resource bundle.
    ///
    /// # Contract
    ///
    /// Exactly return the dependencies you're going to `fetch`! Doing otherwise *will* cause a
    /// panic.
    ///
    /// This method is only executed once,
    /// thus the returned value may never change
    /// (otherwise it has no effect).
    ///
    /// [`ResourceId`]: struct.ResourceId.html
    fn writes(&self) -> Vec<ResourceId>;
}

impl Accessor for () {
    fn try_new() -> Option<Self> {
        None
    }
    fn reads(&self) -> Vec<ResourceId> {
        Vec::new()
    }
    fn writes(&self) -> Vec<ResourceId> {
        Vec::new()
    }
}

impl<T: ?Sized> Accessor for PhantomData<T> {
    fn try_new() -> Option<Self> {
        None
    }
    fn reads(&self) -> Vec<ResourceId> {
        Vec::new()
    }
    fn writes(&self) -> Vec<ResourceId> {
        Vec::new()
    }
}

/// Either an `Accessor` of the system `T` or a reference to it.
pub enum AccessorCow<'b, T>
where
    AccessorTy<T>: 'b,
    T: System + ?Sized,
{
    /// A reference to an accessor.
    Ref(&'b AccessorTy<T>),
    /// An owned accessor.
    Owned(AccessorTy<T>),
}

impl<'b, T> Deref for AccessorCow<'b, T>
where
    AccessorTy<T>: 'b,
    T: System + ?Sized + 'b,
{
    type Target = AccessorTy<T>;

    fn deref(&self) -> &AccessorTy<T> {
        match *self {
            AccessorCow::Ref(r) => &*r,
            AccessorCow::Owned(ref o) => o,
        }
    }
}

type AccessorTy<T> = <<T as System>::SystemData as DynamicSystemData>::Accessor;

/// Trait for fetching data and running systems. Automatically implemented for systems.
pub trait RunNow {
    /// Runs the system now.
    ///
    /// # Panics
    ///
    /// Panics if the system tries to fetch resources
    /// which are borrowed in an incompatible way already
    /// (tries to read from a resource which is already written to or
    /// tries to write to a resource which is read from).
    fn run_now(&mut self, res: &Resources);

    /// Sets up `Resources` for a later call to `run_now`.
    fn setup(&mut self, res: &mut Resources);
}

impl<T> RunNow for T
where
    T: System,
{
    fn run_now(&mut self, res: &Resources) {
        let data = T::SystemData::fetch(&self.accessor(), res);
        self.run(data.into_inner());
    }

    fn setup(&mut self, res: &mut Resources) {
        T::setup(self, res);
    }
}

#[repr(u8)]
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum RunningTime {
    VeryShort = 1,
    Short = 2,
    Average = 3,
    Long = 4,
    VeryLong = 5,
}

/// A `System`, executed with a set of required [`Resource`]s.
///
/// [`Resource`]: trait.Resource.html
pub trait System {
    /// The resource bundle required to execute this system.
    ///
    /// You will mostly use a tuple of system data (which also implements `SystemData`).
    /// You can also create such a resource bundle by simply deriving `SystemData` for a struct.
    ///
    /// Every `SystemData` is also a `DynamicSystemData`.
    type SystemData: DynamicSystemData;

    /// Executes the system with the required system
    /// data.
    fn run(&mut self, data: Self::SystemData);

    /// Returns a hint how long the system needs for running.
    /// This is used to optimize the way they're executed (might
    /// allow more parallelization).
    ///
    /// Defaults to `RunningTime::Average`.
    fn running_time(&self) -> RunningTime {
        RunningTime::Average
    }

    /// Return the accessor from the [`SystemData`].
    fn accessor<'b>(&'b self) -> AccessorCow<'b, Self> {
        AccessorCow::Owned(
            AccessorTy::<Self>::try_new()
                .expect("Missing implementation for `accessor`"),
        )
    }

    /// Sets up the `Resources` using `Self::SystemData::setup`.
    fn setup(&mut self, res: &mut Resources) {
        <Self::SystemData as DynamicSystemData>::setup(&self.accessor(), res)
    }
}

/// A fetch that tracks a lifetime.
pub struct SystemFetch<'r, T> {
    value: T,
    marker: PhantomData<& 'r ()>,
}

impl<'r, T> SystemFetch<'r, T> {
    /// Construct a system fetch from a value.
    pub fn from(value: T) -> SystemFetch<'r, T> {
        SystemFetch {
            value,
            marker: PhantomData,
        }
    }

    /// Convert into inner value.
    ///
    /// Unsafe because it discards the marker lifetime.
    pub fn into_inner(self) -> T {
        self.value
    }
}

/// A static system data that can specify its dependencies at statically (at compile-time).
/// Most system data is a `SystemData`, the `DynamicSystemData` type is only needed for very special
/// setups.
pub trait SystemData {
    /// Sets up the system data for fetching it from the `Resources`.
    fn setup(res: &mut Resources);

    /// Fetches the system data from `Resources`. Note that this is only specified for one concrete
    /// lifetime `'a`, you need to implement the `SystemData` trait for every possible
    /// lifetime.
    fn fetch<'r>(res: &'r Resources) -> SystemFetch<'r, Self>
        where Self: Sized;

    /// Returns all read dependencies as fetched from `Self::fetch`.
    ///
    /// Please note that returning wrong dependencies can lead to a panic.
    fn reads() -> Vec<ResourceId>;

    /// Returns all write dependencies as fetched from `Self::fetch`.
    ///
    /// Please note that returning wrong dependencies can lead to a panic.
    fn writes() -> Vec<ResourceId>;
}

impl<T> DynamicSystemData for T
where
    T: SystemData,
{
    type Accessor = StaticAccessor<T>;

    fn setup(_: &StaticAccessor<T>, res: &mut Resources) {
        T::setup(res);
    }

    fn fetch<'r>(_: &StaticAccessor<T>, res: &'r Resources) -> SystemFetch<'r, Self> {
        T::fetch(res)
    }
}

impl SystemData for () {
    fn setup(_: &mut Resources) {}

    fn fetch<'r>(_: &'r Resources) -> SystemFetch<'r, Self> {
        SystemFetch::from(())
    }

    fn reads() -> Vec<ResourceId> {
        Vec::new()
    }

    fn writes() -> Vec<ResourceId> {
        Vec::new()
    }
}

/// The static accessor that is used for `SystemData`.
#[derive(Default)]
pub struct StaticAccessor<T> {
    marker: PhantomData<fn() -> T>,
}

impl<T> Accessor for StaticAccessor<T>
where
    T: SystemData,
{
    fn try_new() -> Option<Self> {
        Some(StaticAccessor {
            marker: PhantomData,
        })
    }
    fn reads(&self) -> Vec<ResourceId> {
        T::reads()
    }
    fn writes(&self) -> Vec<ResourceId> {
        T::writes()
    }
}

/// A struct implementing system data indicates that it bundles some resources which are required
/// for the execution.
///
/// This is the more flexible, but complex variant of `SystemData`.
pub trait DynamicSystemData {
    /// The accessor of the `SystemData`, which specifies the read and write dependencies and does
    /// the fetching.
    type Accessor: Accessor;

    /// Sets up `Resources` for fetching this system data.
    fn setup(accessor: &Self::Accessor, res: &mut Resources);

    /// Creates a new resource bundle
    /// by fetching the required resources
    /// from the [`Resources`] struct.
    ///
    /// # Contract
    ///
    /// Only fetch the resources you returned from `reads` / `writes`!
    ///
    /// # Panics
    ///
    /// This function may panic if the above contract is violated.
    /// This function may panic if the resource doesn't exist. This is only the case if either
    /// `setup` was not called or it didn't insert any fallback value.
    ///
    /// [`Resources`]: trait.Resources.html
    fn fetch<'r>(access: &Self::Accessor, res: &'r Resources) -> SystemFetch<'r, Self>
        where Self: Sized;
}

impl<T: ?Sized> SystemData for PhantomData<T> {
    fn setup(_: &mut Resources) {}

    fn fetch<'r>(_: &'r Resources) -> SystemFetch<'r, Self> {
        SystemFetch::from(PhantomData)
    }

    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }
}

macro_rules! impl_data {
    ( $($ty:ident),* ) => {
        impl<$($ty),*> SystemData for ( $( $ty , )* )
            where $( $ty : SystemData ),*
            {
                fn setup(res: &mut Resources) {
                    #![allow(unused_variables)]

                    $(
                        <$ty as SystemData>::setup(&mut *res);
                     )*
                }

                fn fetch<'r>(res: &'r Resources) -> SystemFetch<'r, Self> {
                    #![allow(unused_variables)]

                    SystemFetch::from(
                        ( $( <$ty as SystemData>::fetch(res).into_inner(), )* )
                    )
                }

                fn reads() -> Vec<ResourceId> {
                    #![allow(unused_mut)]

                    let mut r = Vec::new();

                    $( {
                        let mut reads = <$ty as SystemData>::reads();
                        r.append(&mut reads);
                    } )*

                    r
                }

                fn writes() -> Vec<ResourceId> {
                    #![allow(unused_mut)]

                    let mut r = Vec::new();

                    $( {
                        let mut writes = <$ty as SystemData>::writes();
                        r.append(&mut writes);
                    } )*

                    r
                }
            }
    };
}

mod impl_data {
    #![cfg_attr(rustfmt, rustfmt_skip)]

    use super::*;

    impl_data!(A);
    impl_data!(A, B);
    impl_data!(A, B, C);
    impl_data!(A, B, C, D);
    impl_data!(A, B, C, D, E);
    impl_data!(A, B, C, D, E, F);
    impl_data!(A, B, C, D, E, F, G);
    impl_data!(A, B, C, D, E, F, G, H);
    impl_data!(A, B, C, D, E, F, G, H, I);
    impl_data!(A, B, C, D, E, F, G, H, I, J);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
    impl_data!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
}
