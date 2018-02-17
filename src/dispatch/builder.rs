use std::fmt;

use fxhash::FxHashMap;

use dispatch::Dispatcher;
use dispatch::dispatcher::{SystemId, ThreadLocal};
use dispatch::stage::StagesBuilder;
use system::{RunNow, System};

/// Builder for the [`Dispatcher`].
///
/// [`Dispatcher`]: struct.Dispatcher.html
///
/// ## Barriers
///
/// Barriers are a way of sequentializing parts of
/// the system execution. See `add_barrier()`/`with_barrier()`.
///
/// ## Examples
///
/// This is how you create a dispatcher with
/// a shared thread pool:
///
/// ```rust
/// # #![allow(unused)]
/// #
/// # extern crate shred;
/// # #[macro_use]
/// # extern crate shred_derive;
/// # use shred::{Dispatcher, DispatcherBuilder, Fetch, System};
/// # #[derive(Debug, Default)] struct Res;
/// # #[derive(SystemData)] #[allow(unused)] struct Data<'a> { a: Fetch<'a, Res> }
/// # struct Dummy;
/// # impl<'a> System<'a> for Dummy {
/// #   type SystemData = Data<'a>;
/// #
/// #   fn run(&mut self, _: Data<'a>) {}
/// # }
/// #
/// # fn main() {
/// # let system_a = Dummy;
/// # let system_b = Dummy;
/// # let system_c = Dummy;
/// # let system_d = Dummy;
/// # let system_e = Dummy;
/// let dispatcher: Dispatcher = DispatcherBuilder::new()
///     .with(system_a, "a", &[])
///     .with(system_b, "b", &["a"]) // b depends on a
///     .with(system_c, "c", &["a"]) // c also depends on a
///     .with(system_d, "d", &[])
///     .with(system_e, "e", &["c", "d"]) // e executes after c and d are finished
///     .build();
/// # }
/// ```
///
/// Systems can be conditionally added by using the `add_` functions:
///
/// ```rust
/// # #![allow(unused)]
/// #
/// # extern crate shred;
/// # #[macro_use]
/// # extern crate shred_derive;
/// # use shred::{Dispatcher, DispatcherBuilder, Fetch, System};
/// # #[derive(Debug, Default)] struct Res;
/// # #[derive(SystemData)] #[allow(unused)] struct Data<'a> { a: Fetch<'a, Res> }
/// # struct Dummy;
/// # impl<'a> System<'a> for Dummy {
/// #   type SystemData = Data<'a>;
/// #
/// #   fn run(&mut self, _: Data<'a>) {}
/// # }
/// #
/// # fn main() {
/// # let b_enabled = true;
/// # let system_a = Dummy;
/// # let system_b = Dummy;
/// let mut builder = DispatcherBuilder::new()
///     .with(system_a, "a", &[]);
/// 
/// if b_enabled {
///    builder.add(system_b, "b", &[]);
/// }
/// 
/// let dispatcher = builder.build();
/// # }
/// ```
///
#[derive(Default)]
pub struct DispatcherBuilder<'a, 'b> {
    current_id: usize,
    map: FxHashMap<String, SystemId>,
    stages_builder: StagesBuilder<'a>,
    thread_local: ThreadLocal<'b>,
    #[cfg(feature = "parallel")]
    thread_pool:
        Option<::std::sync::Arc<::rayon::ThreadPool>>,
}

impl<'a, 'b> DispatcherBuilder<'a, 'b> {
    /// Creates a new `DispatcherBuilder` by using the `Default` implementation.
    ///
    /// The default behaviour is to create a thread pool on `finish`.
    /// If you already have a rayon `ThreadPool`, it's highly recommended to configure
    /// this builder to use it with `with_pool` instead.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a new system with a given name and a list of dependencies.
    /// Please note that the dependency should be added before
    /// you add the depending system.
    ///
    /// If you want to register systems which can not be specified as
    /// dependencies, you can use `""` as their name, which will not panic
    /// (using another name twice will).
    ///
    /// Same as [`add()`](struct.DispatcherBuilder.html#method.add), but
    /// returns `self` to enable method chaining.
    ///
    /// # Panics
    ///
    /// * if the specified dependency does not exist
    /// * if a system with the same name was already registered.
    pub fn with<T>(mut self, system: T, name: &str, dep: &[&str]) -> Self
    where
        T: for<'c> System<'c> + Send + 'a,
    {
        self.add(system, name, dep);
        
        self
    }

    /// Adds a new system with a given name and a list of dependencies.
    /// Please note that the dependency should be added before
    /// you add the depending system.
    ///
    /// If you want to register systems which can not be specified as
    /// dependencies, you can use `""` as their name, which will not panic
    /// (using another name twice will).
    ///
    /// # Panics
    ///
    /// * if the specified dependency does not exist
    /// * if a system with the same name was already registered.
    pub fn add<T>(&mut self, system: T, name: &str, dep: &[&str])
    where
        T: for<'c> System<'c> + Send + 'a,
    {
        use std::collections::hash_map::Entry;

        let id = self.next_id();

        let dependencies = dep.iter()
            .map(|x| {
                *self.map
                    .get(*x)
                    .expect(&format!("No such system registered (\"{}\")", *x))
            })
            .collect();

        if name != "" {
            if let Entry::Vacant(e) = self.map.entry(name.to_owned()) {
                e.insert(id);
            } else {
                panic!(
                    "Cannot insert multiple systems with the same name (\"{}\")",
                    name
                );
            }
        }

        self.stages_builder.insert(dependencies, id, system);
    }

    /// Adds a new thread local system.
    ///
    /// Please only use this if your struct is not `Send` and `Sync`.
    ///
    /// Thread-local systems are dispatched in-order.
    /// 
    /// Same as
    /// [`add_thread_local()`](struct.DispatcherBuilder.html#method.add_thread_local),
    /// but returns `self` to enable method chaining.
    pub fn with_thread_local<T>(mut self, system: T) -> Self
    where
        T: for<'c> RunNow<'c> + 'b,
    {
        self.add_thread_local(system);
        
        self
    }

    /// Adds a new thread local system.
    ///
    /// Please only use this if your struct is not `Send` and `Sync`.
    ///
    /// Thread-local systems are dispatched in-order.
    pub fn add_thread_local<T>(&mut self, system: T)
    where
        T: for<'c> RunNow<'c> + 'b,
    {
        self.thread_local.push(Box::new(system));
    }

    /// Inserts a barrier which assures that all systems
    /// added before the barrier are executed before the ones
    /// after this barrier.
    ///
    /// Does nothing if there were no systems added
    /// since the last call to `add_barrier()`/`with_barrier()`.
    ///
    /// Thread-local systems are not affected by barriers;
    /// they're always executed at the end.
    ///
    /// Same as
    /// [`add_barrier()`](struct.DispatcherBuilder.html#method.add_barrier),
    /// but returns `self` to enable method chaining.
    pub fn with_barrier(mut self) -> Self {
        self.add_barrier();
        
        self
    }

    /// Inserts a barrier which assures that all systems
    /// added before the barrier are executed before the ones
    /// after this barrier.
    ///
    /// Does nothing if there were no systems added
    /// since the last call to `add_barrier()`/`with_barrier()`.
    ///
    /// Thread-local systems are not affected by barriers;
    /// they're always executed at the end.
    pub fn add_barrier(&mut self) {
        self.stages_builder.add_barrier();
    }

    /// Attach a rayon thread pool to the builder
    /// and use that instead of creating one.
    ///
    /// Same as
    /// [`add_pool()`](struct.DispatcherBuilder.html#method.add_pool),
    /// but returns `self` to enable method chaining.
    #[cfg(feature = "parallel")]
    pub fn with_pool(mut self, pool: ::std::sync::Arc<::rayon::ThreadPool>) -> Self {
        self.add_pool(pool);
        
        self
    }

    /// Attach a rayon thread pool to the builder
    /// and use that instead of creating one.
    #[cfg(feature = "parallel")]
    pub fn add_pool(&mut self, pool: ::std::sync::Arc<::rayon::ThreadPool>) {
        self.thread_pool = Some(pool);
    }

    /// Prints the equivalent system graph
    /// that can be easily used to get the graph using the `seq!` and `par!`
    /// macros. This is only recommended for advanced users.
    pub fn print_par_seq(&self) {
        println!("{:#?}", self);
    }

    /// Builds the `Dispatcher`.
    ///
    /// In the future, this method will
    /// precompute useful information in
    /// order to speed up dispatching.
    pub fn build(self) -> Dispatcher<'a, 'b> {
        use dispatch::dispatcher::new_dispatcher;

        #[cfg(feature = "parallel")]
        let d = new_dispatcher(
            self.stages_builder.build(),
            self.thread_local,
            self.thread_pool.unwrap_or_else(Self::create_thread_pool),
        );

        #[cfg(not(feature = "parallel"))]
        let d = new_dispatcher(self.stages_builder.build(), self.thread_local);

        d
    }

    fn next_id(&mut self) -> SystemId {
        let id = self.current_id;
        self.current_id += 1;

        SystemId(id)
    }

    #[cfg(feature = "parallel")]
    fn create_thread_pool() -> ::std::sync::Arc<::rayon::ThreadPool> {
        use std::sync::Arc;
        use rayon::ThreadPoolBuilder;

        Arc::new(
            ThreadPoolBuilder::new().build().expect("Invalid configuration"),
        )
    }
}

#[cfg(feature = "parallel")]
impl<'b> DispatcherBuilder<'static, 'b> {
    /// Builds an async dispatcher.
    ///
    /// It does not allow non-static types and
    /// accepts a `Resource` struct.
    pub fn build_async<R>(self, res: R) -> ::dispatch::async::AsyncDispatcher<'b, R> {
        use dispatch::async::new_async;

        new_async(
            res,
            self.stages_builder.build(),
            self.thread_local,
            self.thread_pool.unwrap_or_else(Self::create_thread_pool),
        )
    }
}

impl<'a, 'b> fmt::Debug for DispatcherBuilder<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.stages_builder.write_par_seq(f, &self.map)
    }
}
