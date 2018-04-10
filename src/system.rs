use std::marker::PhantomData;

use {ResourceId, Resources};

/// Trait for fetching data and running systems. Automatically implemented for systems.
pub trait RunNow<'a> {
    /// Runs the system now.
    ///
    /// # Panics
    ///
    /// Panics if the system tries to fetch resources
    /// which are borrowed in an incompatible way already
    /// (tries to read from a resource which is already written to or
    /// tries to write to a resource which is read from).
    fn run_now(&mut self, res: &'a Resources);

    /// Sets up `Resources` for a later call to `run_now`.
    fn setup(&mut self, res: &mut Resources);
}

impl<'a, T> RunNow<'a> for T
where
T: System<'a>,
{
    fn run_now(&mut self, res: &'a Resources) {
        let data = T::SystemData::fetch(res);
        self.run(data);
    }

    fn setup(&mut self, res: &mut Resources) {
        T::setup(res);
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

/// A `System`, executed with a
/// set of required [`Resource`]s.
///
/// [`Resource`]: trait.Resource.html
pub trait System<'a> {
    /// The resource bundle required
    /// to execute this system.
    ///
    /// To create such a resource bundle,
    /// simple derive `SystemData` for it.
    type SystemData: SystemData<'a>;

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
    fn accessor(&self) -> <<Self as System<'a>>::SystemData as SystemData<'a>>::Accessor {
        <Self::SystemData>::Accessor::try_new().expect("Missing implementation for `accessor`")
    }

    /// Sets up the `Resources` using `Self::SystemData::setup`.
    fn setup(res: &mut Resources) {
        <Self::SystemData as SystemData>::setup(res)
    }
}

/// A struct implementing
/// static system data adds a
/// static accessor.
pub trait StaticSystemData<'a> {
    fn setup(res: &mut Resources);

    fn fetch(res: &'a Resources) -> Self;

    fn reads() -> Vec<ResourceId>;

    fn writes() -> Vec<ResourceId>;
}

impl<'a, T> SystemData<'a> for T where T: StaticSystemData<'a> {
    type Accessor = StaticAccessor<T>;
}

impl<'a> StaticSystemData<'a> for () {
    fn setup(res: &mut Resources) {
    }

    fn fetch(res: &'a Resources) -> Self {
        ()
    }

    fn reads(&self) -> Vec<ResourceId> {
        Vec::new()
    }
    fn writes(&self) -> Vec<ResourceId> {
        Vec::new()
    }
}

impl<'a, T> Accessor for StaticAccessor<T> where T: StaticSystemData<'a> {
    fn reads(&self) -> Vec<ResourceId> {
        T::reads()
    }

    fn writes(&self) -> Vec<ResourceId> {
        T::writes()
    }
}

/// A struct implementing
/// system data indicates that it
/// bundles some resources which are
/// required for the execution.
pub trait SystemData<'a> {
    type Accessor: Accessor;

    /// Sets up `Resources` for fetching this system data.
    fn setup(res: &mut Resources);

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
    fn fetch(access: &Self::Accessor, res: &'a Resources) -> Self;
}

impl<'a, T: ?Sized> SystemData<'a> for PhantomData<T> {
    type Accessor = StaticAccessor<()>;

    fn setup(_: &mut Resources) {}

    fn fetch(_: &Self::Accessor, _: &'a Resources) -> Self {
        PhantomData
    }
}

macro_rules! impl_data {
    ( $($ty:ident),* ) => {
        impl<'a, $($ty),*> SystemData<'a> for ( $( $ty , )* )
            where $( $ty : SystemData<'a> ),*
            {
                type Accessor = StaticAccessor<$($ty),*>;

                fn setup(res: &mut Resources) {
                    #![allow(unused_variables)]

                    $(
                        <$ty as SystemData>::setup(&mut *res);
                     )*
                }

                fn fetch(access: &Self::Accessor, res: &'a Resources) -> Self {
                    #![allow(unused_variables)]

                    ( $( <$ty as SystemData<'a>>::fetch(access, res), )* )
                }
            }

        impl<'a, $($ty),*> StaticSystemData<'a> for ( $( $ty , )* )
            where $( $ty : StaticSystemData<'a> ),*
            {
                fn setup(res: &mut Resources) {
                    #![allow(unused_variables)]

                    $(
                        <$ty as StaticSystemData>::setup(&mut *res);
                     )*
                }

                fn fetch(res: &'a Resources) -> Self {
                    #![allow(unused_variables)]

                    ( $( <$ty as StaticSystemData<'a>>::fetch(res), )* )
                }

                fn reads() -> Vec<ResourceId> {
                    #![allow(unused_mut)]

                    let mut r = Vec::new();

                    $( {
                        let mut reads = <$ty as StaticSystemData>::reads();
                        r.append(&mut reads);
                    } )*

                    r
                }

                fn writes() -> Vec<ResourceId> {
                    #![allow(unused_mut)]

                    let mut r = Vec::new();

                    $( {
                        let mut writes = <$ty as StaticSystemData>::writes();
                        r.append(&mut writes);
                    } )*

                    r
                }
            }

        impl<$($ty),*> StaticAccessor<$($ty),*> {
            fn reads() -> Vec<ResourceId> {
                #![allow(unused_mut)]

                let mut r = Vec::new();

                $( {
                    let mut reads = <$ty as Accessor>::reads();
                    r.append(&mut reads);
                } )*

                r
            }

            fn writes() -> Vec<ResourceId> {
                #![allow(unused_mut)]

                let mut r = Vec::new();

                $( {
                    let mut writes = <$ty as Accessor>::writes();
                    r.append(&mut writes);
                } )*

                r
            }
        }
    };
}

/// A trait for accessing read/write bundles of [`ResourceId`]s in ['SystemData']. This can be used
/// to create dynamic systems that don't specify what they fetch at compile-time.
pub trait Accessor {
    /// A list of [`ResourceId`]s the bundle
    /// needs read access to in order to
    /// build the target resource bundle.
    ///
    /// # Contract
    ///
    /// Exactly return the dependencies you're
    /// going to `fetch`! Doing otherwise *will*
    /// cause a data race.
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
    /// Exactly return the dependencies you're
    /// going to `fetch`! Doing otherwise *will*
    /// cause a data race.
    ///
    /// This method is only executed once,
    /// thus the returned value may never change
    /// (otherwise it has no effect).
    ///
    /// [`ResourceId`]: struct.ResourceId.html
    fn writes(&self) -> Vec<ResourceId>;
}

impl Accessor for () {
    fn reads(&self) -> Vec<ResourceId> {
        Vec::new()
    }
    fn writes(&self) -> Vec<ResourceId> {
        Vec::new()
    }
}

impl<T: ?Sized> Accessor for PhantomData<T> {
    fn reads(&self) -> Vec<ResourceId> {
        Vec::new()
    }
    fn writes(&self) -> Vec<ResourceId> {
        Vec::new()
    }
}

#[derive(Default)]
pub struct StaticAccessor<T> {
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,

    __: PhantomData<fn() ->T>
}

mod impl_data {
    #![cfg_attr(rustfmt, rustfmt_skip)]

    use super::*;

    impl_data!(A);
    /*
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
       */
}
