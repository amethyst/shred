use {ResourceId, Resources};

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
}

/// A struct implementing
/// system data indicates that it
/// bundles some resources which are
/// required for the execution.
pub trait SystemData<'a> {
    /// Creates a new resource bundle
    /// by fetching the required resources
    /// from the [`Resources`] struct.
    ///
    /// # Contract
    ///
    /// Only fetch the resources you
    /// returned from `reads` / `writes`!
    ///
    /// [`Resources`]: trait.Resources.html
    fn fetch(res: &'a Resources, id: usize) -> Self;

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
    unsafe fn reads(id: usize) -> Vec<ResourceId>;

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
    unsafe fn writes(id: usize) -> Vec<ResourceId>;
}

macro_rules! impl_data {
    ( $($ty:ident),* ) => {
        impl<'a, $($ty),*> SystemData<'a> for ( $( $ty , )* )
            where $( $ty : SystemData<'a> ),*
        {
            fn fetch(res: &'a Resources, id: usize) -> Self {
                #![allow(unused_variables)]

                ( $( <$ty as SystemData<'a>>::fetch(res, id.clone()), )* )
            }

            unsafe fn reads(id: usize) -> Vec<ResourceId> {
                #![allow(unused_mut)]

                let mut r = Vec::new();

                $( {
                        let mut reads = <$ty as SystemData>::reads(id.clone());
                        r.append(&mut reads);
                    } )*

                r
            }

            unsafe fn writes(id: usize) -> Vec<ResourceId> {
                #![allow(unused_mut)]

                let mut r = Vec::new();

                $( {
                        let mut writes = <$ty as SystemData>::writes(id.clone());
                        r.append(&mut writes);
                    } )*

                r
            }
        }
    };
}

impl<'a> SystemData<'a> for () {
    fn fetch(_: &'a Resources, _: usize) -> Self {
        ()
    }

    unsafe fn reads(_: usize) -> Vec<ResourceId> {
        Vec::new()
    }

    unsafe fn writes(_: usize) -> Vec<ResourceId> {
        Vec::new()
    }
}

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
