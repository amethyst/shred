use std::borrow::Borrow;

use res::Resources;
use system::RunNow;
use system::System;

use rayon::{ThreadPool, join};

pub struct Nil;

/// The `par!` macro may be used to easily create a structure
/// which runs things in parallel.
///
/// ## Examples
///
/// ```
/// #[macro_use(par)]
/// extern crate shred;
///
/// # struct SysA;
/// # struct SysB;
/// # struct SysC;
/// # fn main() {
/// par![
///     SysA,
///     SysB,
///     SysC,
/// ]
/// # ;}
/// ```
#[macro_export]
macro_rules! par {
    ($head:expr, $( $tail:expr ,)+) => {
        {
            $crate::Par::new($head)
                $( .with($tail) )*
        }
    };
}

/// The `seq!` macro may be used to easily create a structure
/// which runs things sequentially.
///
/// ## Examples
///
/// ```
/// #[macro_use(seq)]
/// extern crate shred;
///
/// # struct SysA;
/// # struct SysB;
/// # struct SysC;
/// # fn main() {
/// seq![
///     SysA,
///     SysB,
///     SysC,
/// ]
/// # ;}
/// ```
#[macro_export]
macro_rules! seq {
    ($head:expr, $( $tail:expr ,)+) => {
        {
            $crate::Seq::new($head)
                $( .with($tail) )*
        }
    };
}

impl<'a> System<'a> for Nil {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {}
}

/// Runs two tasks in parallel.
/// These two tasks are called `head` and `tail`
/// in the following documentation.
pub struct Par<H, T> {
    head: H,
    tail: T,
}

impl<H> Par<H, Nil> {
    /// Creates a new `Par` struct, with the tail being a no-op.
    pub fn new(head: H) -> Self {
        Par { head, tail: Nil }
    }

    /// Adds `sys` as the second job and returns a new `Par` struct
    /// with the previous struct as head and a no-op tail.
    pub fn with<T>(self, sys: T) -> Par<Par<H, T>, Nil> {
        Par {
            head: Par {
                head: self.head,
                tail: sys,
            },
            tail: Nil,
        }
    }
}

/// A dispatcher intended to be used with
/// `Par` and `Seq`  structures.
///
/// ## Examples
///
/// ```
/// # extern crate rayon;
/// #[macro_use(par, seq)]
/// extern crate shred;
///
/// # use rayon::ThreadPool;
/// #
/// # use shred::{ParSeq, Resources, System};
/// #
/// # macro_rules! impl_sys {
/// #     ($( $id:ident )*) => {
/// #         $(
/// #             impl<'a> ::shred::System<'a> for $id {
/// #                 type SystemData = ();
/// #                 fn run(&mut self, _: Self::SystemData) {}
/// #             }
/// #         )*
/// #     };
/// # }
/// #
/// # struct SysA;
/// # struct SysB;
/// # struct SysC;
/// # struct SysD;
/// # struct SysWithLifetime<'a>(&'a u8);
/// # struct SysLocal(*const u8);
/// #
/// # impl_sys!(SysA SysB SysC SysD SysLocal);
/// #
/// # impl<'a, 'b> System<'a> for SysWithLifetime<'b> {
/// #     type SystemData = ();
/// #
/// #     fn run(&mut self, _: Self::SystemData) {}
/// # }
///
/// # fn main() {
/// # #![cfg_attr(rustfmt, rustfmt_skip)]
/// #
/// # let pool = ThreadPool::new(Default::default()).unwrap();
/// #
/// # let mut res = Resources::new();
/// let x = 5u8;
///
/// let mut dispatcher = ParSeq::new(
///     seq![
///         par![
///             SysA,
///             SysWithLifetime(&x),
///             seq![
///                 SysC,
///                 SysD,
///             ],
///         ],
///         SysB,
///         SysLocal(&x as *const u8),
///     ],
///     &pool,
/// );
///
/// dispatcher.dispatch(&mut res);
/// # }
/// ```
pub struct ParSeq<P, T> {
    run: T,
    pool: P,
}

impl<P, T> ParSeq<P, T>
where
    P: Borrow<ThreadPool>,
    T: for<'a> RunWithPool<'a>,
{
    /// Creates a new `ParSeq` dispatcher.
    /// `run` is usually created by using the `par!` / `seq!`
    /// macros.
    pub fn new(run: T, pool: P) -> Self {
        ParSeq { run, pool }
    }

    /// Dispatches the systems using `res`.
    /// This involves zero virtual cost.
    pub fn dispatch(&mut self, res: &mut Resources) {
        self.run.run(res, self.pool.borrow());
    }
}

pub trait RunWithPool<'a> {
    fn run(&mut self, res: &'a Resources, pool: &ThreadPool);
}

impl<'a, T> RunWithPool<'a> for T
where
    T: System<'a>,
{
    fn run(&mut self, res: &'a Resources, _: &ThreadPool) {
        RunNow::run_now(self, res);
    }
}

impl<'a, H, T> RunWithPool<'a> for Par<H, T>
where
    H: RunWithPool<'a> + Send,
    T: RunWithPool<'a> + Send,
{
    fn run(&mut self, res: &'a Resources, pool: &ThreadPool) {
        let head = &mut self.head;
        let tail = &mut self.tail;

        let head = move || head.run(res, pool);
        let tail = move || tail.run(res, pool);

        if pool.current_thread_index().is_none() {
            pool.join(head, tail);
        } else {
            join(head, tail);
        }
    }
}

/// Runs two tasks sequentially.
/// These two tasks are called `head` and `tail`
/// in the following documentation.
pub struct Seq<H, T> {
    head: H,
    tail: T,
}

impl<H> Seq<H, Nil> {
    /// Creates a new `Seq` struct, with the tail being a no-op.
    pub fn new(head: H) -> Self {
        Seq { head, tail: Nil }
    }

    /// Adds `sys` as the second job and returns a new `Seq` struct
    /// with the previous struct as head and a no-op tail.
    pub fn with<T>(self, sys: T) -> Seq<Seq<H, T>, Nil> {
        Seq {
            head: Seq {
                head: self.head,
                tail: sys,
            },
            tail: Nil,
        }
    }
}

impl<'a, H, T> RunWithPool<'a> for Seq<H, T>
where
    H: RunWithPool<'a>,
    T: RunWithPool<'a>,
{
    fn run(&mut self, res: &'a Resources, pool: &ThreadPool) {
        self.head.run(res, pool);
        self.tail.run(res, pool);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::*;

    #[test]
    fn nested_joins() {
        let pool = ThreadPool::new(Default::default()).unwrap();

        pool.join(|| join(|| join(|| join(|| (), || ()), || ()), || ()), || ());
    }

    #[test]
    fn build_par() {
        let pool = ThreadPool::new(Default::default()).unwrap();

        struct A(Arc<AtomicUsize>);

        impl<'a> System<'a> for A {
            type SystemData = ();

            fn run(&mut self, _: Self::SystemData) {
                self.0.fetch_add(1, Ordering::AcqRel);
            }
        }

        let nr = Arc::new(AtomicUsize::new(0));

        Par::new(A(nr.clone()))
            .with(A(nr.clone()))
            .with(A(nr.clone()))
            .run(&Resources::new(), &pool);

        assert_eq!(nr.load(Ordering::Acquire), 3);

        par![A(nr.clone()), A(nr.clone()),].run(&Resources::new(), &pool);

        assert_eq!(nr.load(Ordering::Acquire), 5);
    }

    #[test]
    fn build_seq() {
        let pool = ThreadPool::new(Default::default()).unwrap();

        struct A(Arc<AtomicUsize>);

        impl<'a> System<'a> for A {
            type SystemData = ();

            fn run(&mut self, _: Self::SystemData) {
                self.0.fetch_add(1, Ordering::AcqRel);
            }
        }

        let nr = Arc::new(AtomicUsize::new(0));

        Seq::new(A(nr.clone()))
            .with(A(nr.clone()))
            .with(A(nr.clone()))
            .run(&Resources::new(), &pool);

        assert_eq!(nr.load(Ordering::Acquire), 3);
    }
}
