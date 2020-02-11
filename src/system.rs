use std::{marker::PhantomData, ops::Deref};

use crate::{ResourceId, World};

/// A trait for accessing read/write multiple resources from a system. This can
/// be used to create dynamic systems that don't specify what they fetch at
/// compile-time.
///
/// For compile-time system data this will all be done for you using
/// `StaticAccessor`.
pub trait Accessor: Sized {
    /// Tries to create a new instance of this type. This one returns `Some` in
    /// case there is a default, otherwise the system needs to override
    /// `System::accessor`.
    fn try_new() -> Option<Self>;

    /// A list of [`ResourceId`]s the bundle
    /// needs read access to in order to
    /// build the target resource bundle.
    ///
    /// # Contract
    ///
    /// Exactly return the dependencies you're going to `fetch`! Doing otherwise
    /// *will* cause a panic.
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
    /// Exactly return the dependencies you're going to `fetch`! Doing otherwise
    /// *will* cause a panic.
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
pub enum AccessorCow<'a, 'b, T>
where
    AccessorTy<'a, T>: 'b,
    T: System<'a> + ?Sized,
    'a: 'b,
{
    /// A reference to an accessor.
    Ref(&'b AccessorTy<'a, T>),
    /// An owned accessor.
    Owned(AccessorTy<'a, T>),
}

impl<'a, 'b, T> Deref for AccessorCow<'a, 'b, T>
where
    AccessorTy<'a, T>: 'b,
    T: System<'a> + ?Sized + 'b,
    'a: 'b,
{
    type Target = AccessorTy<'a, T>;

    fn deref(&self) -> &AccessorTy<'a, T> {
        match *self {
            AccessorCow::Ref(r) => &*r,
            AccessorCow::Owned(ref o) => o,
        }
    }
}

type AccessorTy<'a, T> = <<T as System<'a>>::SystemData as DynamicSystemData<'a>>::Accessor;

/// Trait for fetching data and running systems. Automatically implemented for
/// systems.
pub trait RunNow<'a> {
    /// Runs the system now.
    ///
    /// # Panics
    ///
    /// Panics if the system tries to fetch resources
    /// which are borrowed in an incompatible way already
    /// (tries to read from a resource which is already written to or
    /// tries to write to a resource which is read from).
    fn run_now(&mut self, world: &'a World);

    /// Sets up `World` for a later call to `run_now`.
    fn setup(&mut self, world: &mut World);

    /// Performs clean up that requires resources from the `World`.
    /// This commonly removes components from `world` which depend on external
    /// resources.
    #[allow(clippy::boxed_local)]
    fn dispose(self: Box<Self>, world: &mut World) {
        let _ = world;
    }
}

impl<'a, T> RunNow<'a> for T
where
    T: System<'a>,
{
    fn run_now(&mut self, world: &'a World) {
        let data = T::SystemData::fetch(&self.accessor(), world);
        self.run(data);
    }

    fn setup(&mut self, world: &mut World) {
        T::setup(self, world);
    }

    fn dispose(self: Box<Self>, world: &mut World) {
        T::dispose(*self, world);
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
pub trait System<'a> {
    /// The resource bundle required to execute this system.
    ///
    /// You will mostly use a tuple of system data (which also implements
    /// `SystemData`). You can also create such a resource bundle by simply
    /// deriving `SystemData` for a struct.
    ///
    /// Every `SystemData` is also a `DynamicSystemData`.
    type SystemData: DynamicSystemData<'a>;

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
    fn accessor<'b>(&'b self) -> AccessorCow<'a, 'b, Self> {
        AccessorCow::Owned(
            AccessorTy::<'a, Self>::try_new().expect("Missing implementation for `accessor`"),
        )
    }

    /// Sets up the `World` using `Self::SystemData::setup`.
    fn setup(&mut self, world: &mut World) {
        <Self::SystemData as DynamicSystemData>::setup(&self.accessor(), world)
    }

    /// Performs clean up that requires resources from the `World`.
    /// This commonly removes components from `world` which depend on external
    /// resources.
    fn dispose(self, world: &mut World)
    where
        Self: Sized,
    {
        let _ = world;
    }
}

/// A static system data that can specify its dependencies at statically (at
/// compile-time). Most system data is a `SystemData`, the `DynamicSystemData`
/// type is only needed for very special setups.
///
/// You can derive this using the `#[derive(SystemData)]` macro provided by
/// `shred-derive`. That is as simple as enabling the `shred-derive` feature.
///
/// # Examples
///
/// ```rust
/// use shred::{Read, ResourceId, SystemData, World, Write};
///
/// #[derive(Default)]
/// pub struct Clock;
/// #[derive(Default)]
/// pub struct Timer;
///
/// // This will implement `SystemData` for `MySystemData`.
/// // Please note that this will only work if `SystemData`, `World` and `ResourceId` are included.
/// # #[cfg(feature = "shred-derive")]
/// #[derive(SystemData)]
/// pub struct MySystemData<'a> {
///     pub clock: Read<'a, Clock>,
///     pub timer: Write<'a, Timer>,
/// }
/// #
/// # // The following is required for the snippet to compile without the `shred-derive` feature.
/// #
/// # #[cfg(not(feature = "shred-derive"))]
/// # struct MySystemData<'a> {
/// #     pub clock: Read<'a, Clock>,
/// #     pub timer: Write<'a, Timer>,
/// # }
/// #
/// # #[cfg(not(feature = "shred-derive"))]
/// # impl<'a> SystemData<'a> for MySystemData<'a> {
/// #     fn setup(world: &mut World) {
/// #         Read::<'_, Clock>::setup(world);
/// #         Write::<'_, Timer>::setup(world);
/// #     }
/// #
/// #     fn fetch(world: &'a World) -> Self {
/// #         Self {
/// #             clock: Read::<'_, Clock>::fetch(world),
/// #             timer: Write::<'_, Timer>::fetch(world),
/// #         }
/// #     }
/// #
/// #     fn reads() -> Vec<ResourceId> {
/// #         Read::<'_, Clock>::reads()
/// #     }
/// #
/// #     fn writes() -> Vec<ResourceId> {
/// #         Write::<'_, Timer>::writes()
/// #     }
/// # }
/// ```
pub trait SystemData<'a> {
    /// Sets up the system data for fetching it from the `World`.
    fn setup(world: &mut World);

    /// Fetches the system data from `World`. Note that this is only specified
    /// for one concrete lifetime `'a`, you need to implement the
    /// `SystemData` trait for every possible lifetime.
    fn fetch(world: &'a World) -> Self;

    /// Returns all read dependencies as fetched from `Self::fetch`.
    ///
    /// Please note that returning wrong dependencies can lead to a panic.
    fn reads() -> Vec<ResourceId>;

    /// Returns all write dependencies as fetched from `Self::fetch`.
    ///
    /// Please note that returning wrong dependencies can lead to a panic.
    fn writes() -> Vec<ResourceId>;
}

impl<'a, T> DynamicSystemData<'a> for T
where
    T: SystemData<'a>,
{
    type Accessor = StaticAccessor<T>;

    fn setup(_: &StaticAccessor<T>, world: &mut World) {
        T::setup(world);
    }

    fn fetch(_: &StaticAccessor<T>, world: &'a World) -> Self {
        T::fetch(world)
    }
}

impl<'a> SystemData<'a> for () {
    fn setup(_: &mut World) {}

    fn fetch(_: &'a World) -> Self {}

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

impl<'a, T> Accessor for StaticAccessor<T>
where
    T: SystemData<'a>,
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

/// A struct implementing system data indicates that it bundles some resources
/// which are required for the execution.
///
/// This is the more flexible, but complex variant of `SystemData`.
pub trait DynamicSystemData<'a> {
    /// The accessor of the `SystemData`, which specifies the read and write
    /// dependencies and does the fetching.
    type Accessor: Accessor;

    /// Sets up `World` for fetching this system data.
    fn setup(accessor: &Self::Accessor, world: &mut World);

    /// Creates a new resource bundle
    /// by fetching the required resources
    /// from the [`World`] struct.
    ///
    /// # Contract
    ///
    /// Only fetch the resources you returned from `reads` / `writes`!
    ///
    /// # Panics
    ///
    /// This function may panic if the above contract is violated.
    /// This function may panic if the resource doesn't exist. This is only the
    /// case if either `setup` was not called or it didn't insert any
    /// fallback value.
    ///
    /// [`World`]: trait.World.html
    fn fetch(access: &Self::Accessor, world: &'a World) -> Self;
}

impl<'a, T: ?Sized> SystemData<'a> for PhantomData<T> {
    fn setup(_: &mut World) {}

    fn fetch(_: &World) -> Self {
        PhantomData
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
        impl<'a, $($ty),*> SystemData<'a> for ( $( $ty , )* )
            where $( $ty : SystemData<'a> ),*
            {
                fn setup(world: &mut World) {
                    #![allow(unused_variables)]

                    $(
                        <$ty as SystemData>::setup(&mut *world);
                     )*
                }

                fn fetch(world: &'a World) -> Self {
                    #![allow(unused_variables)]

                    ( $( <$ty as SystemData<'a>>::fetch(world), )* )
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
