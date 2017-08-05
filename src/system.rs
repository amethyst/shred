use {ResourceId, Resources};

pub trait Prefetch<'a> {
    type Data: SystemData<'a>;

    fn prefetch(res: &'a Resources) -> Self;

    fn fetch(&mut self) -> Self::Data;
}

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
}

impl<'a, T> RunNow<'a> for T
    where T: System<'a>
{
    fn run_now(&mut self, res: &'a Resources) {
        let data = self.fetch_data(res);
        self.run(data);
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

    /// Fetch the data for `run`. This allows to store a reference
    /// to the cell in the system, so no `HashMap` access is required.
    fn fetch_data(&mut self, res: &'a Resources) -> Self::SystemData {
        Self::SystemData::fetch(res, 0)
    }

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
}

/// A struct implementing
/// system data indicates that it
/// bundles some resources which are
/// required for the execution.
pub trait SystemData<'a> {
    /// The prefetched version of this data.
    type Prefetch: Prefetch<'a>;

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
    fn reads(id: usize) -> Vec<ResourceId>;

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
    fn writes(id: usize) -> Vec<ResourceId>;
}

macro_rules! impl_data {
    ( $($ty:ident . $idx:tt),* ) => {
        impl<'a, $($ty),*> Prefetch<'a> for ( $( $ty , )* )
            where $( $ty : Prefetch<'a> ),*
        {
            type Data = ( $( <$ty as Prefetch<'a>>::Data ),* );

            fn prefetch(res: &'a Resources) -> Self {
                ( $( <$ty as Prefetch<'a>>::prefetch(res) , )* )
            }

            fn fetch(&mut self) -> Self::Data {
                ( $( <$ty as Prefetch<'a>>::fetch(&mut self. $idx ) ),* )
            }
        }

        impl<'a, $($ty),*> SystemData<'a> for ( $( $ty , )* )
            where $( $ty : SystemData<'a> ),*
        {
            type Prefetch = ( $( <$ty as SystemData<'a>>::Prefetch ),* );

            fn fetch(res: &'a Resources, id: usize) -> Self {
                #![allow(unused_variables)]

                ( $( <$ty as SystemData<'a>>::fetch(res, id.clone()), )* )
            }

            fn reads(id: usize) -> Vec<ResourceId> {
                #![allow(unused_mut)]

                let mut r = Vec::new();

                $( {
                        let mut reads = <$ty as SystemData>::reads(id.clone());
                        r.append(&mut reads);
                    } )*

                r
            }

            fn writes(id: usize) -> Vec<ResourceId> {
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

impl<'a> Prefetch<'a> for () {
    type Data = ();

    fn prefetch(_: &'a Resources) -> Self {
        ()
    }

    fn fetch(&mut self) -> Self::Data {
        ()
    }
}

impl<'a> SystemData<'a> for () {
    type Prefetch = ();

    fn fetch(_: &'a Resources, _: usize) -> Self {
        ()
    }

    fn reads(_: usize) -> Vec<ResourceId> {
        Vec::new()
    }

    fn writes(_: usize) -> Vec<ResourceId> {
        Vec::new()
    }
}

mod impl_data {
    #![cfg_attr(rustfmt, rustfmt_skip)]

    use super::*;

    impl_data!(A.0);
    impl_data!(A.0, B.1);
    impl_data!(A.0, B.1, C.2);
    impl_data!(A.0, B.1, C.2, D.3);
    impl_data!(A.0, B.1, C.2, D.3, E.4);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17, S.18);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17, S.18, T.19);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17, S.18, T.19, U.20);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17, S.18, T.19, U.20, V.21);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17, S.18, T.19, U.20, V.21, W.22);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17, S.18, T.19, U.20, V.21, W.22, X.23);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17, S.18, T.19, U.20, V.21, W.22, X.23, Y.24);
    impl_data!(A.0, B.1, C.2, D.3, E.4, F.5, G.6, H.7, I.8, J.9, K.10, L.11, M.12, N.13, O.14, P.15, Q.16, R.17, S.18, T.19, U.20, V.21, W.22, X.23, Y.24, Z.25);
}
