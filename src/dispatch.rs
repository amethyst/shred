use std::sync::Arc;

use fnv::FnvHashMap;
use rayon::{Configuration, Scope, ThreadPool, scope};

use bitset::AtomicBitSet;
use {ResourceId, Resources, System, SystemData};

#[derive(Default)]
struct Dependencies {
    dependencies: Vec<Vec<usize>>,
    rev_reads: FnvHashMap<ResourceId, Vec<usize>>,
    rev_writes: FnvHashMap<ResourceId, Vec<usize>>,
    reads: Vec<Vec<ResourceId>>,
    writes: Vec<Vec<ResourceId>>,
}

impl Dependencies {
    fn add(&mut self,
           id: usize,
           reads: Vec<ResourceId>,
           writes: Vec<ResourceId>,
           dependencies: Vec<usize>) {
        for read in &reads {
            self.rev_reads.entry(*read).or_insert(Vec::new()).push(id);

            self.rev_writes.entry(*read).or_insert(Vec::new());
        }

        for write in &writes {
            self.rev_reads.entry(*write).or_insert(Vec::new());

            self.rev_writes.entry(*write).or_insert(Vec::new()).push(id);
        }

        self.reads.push(reads);
        self.writes.push(writes);
        self.dependencies.push(dependencies);
    }
}

/// The dispatcher struct, allowing
/// systems to be executed in parallel.
pub struct Dispatcher<'c, 't, C> {
    dependencies: Dependencies,
    ready: Vec<usize>,
    running: AtomicBitSet,
    systems: Vec<SystemInfo<'c, 't, C>>,
    thread_pool: Arc<ThreadPool>,
}

impl<'c, 't, C> Dispatcher<'c, 't, C>
    where C: Clone + Send + 'c
{
    /// Dispatches the systems given the
    /// resources to operate on.
    ///
    /// This operation blocks the
    /// executing thread.
    pub fn dispatch(&mut self, res: &mut Resources, context: C) {
        let dependencies = &self.dependencies;
        let ready = self.ready.clone();
        let running = &self.running;
        let systems = &mut self.systems;

        self.thread_pool
            .install(|| {
                         scope(move |scope| {
                    Self::dispatch_inner(dependencies, ready, res, running, scope, systems, context)
                })
                     });

        self.running.clear();
    }

    fn dispatch_inner<'s>(dependencies: &Dependencies,
                          mut ready: Vec<usize>,
                          res: &'s mut Resources,
                          running: &'s AtomicBitSet,
                          scope: &Scope<'s>,
                          systems: &'s mut Vec<SystemInfo<C>>,
                          context: C)
        where 'c: 's
    {
        let mut start_count = 0;
        let num_systems = systems.len();
        let mut systems: Vec<_> = systems.iter_mut().map(|x| Some(x)).collect();

        while start_count < num_systems {
            if let Some(index) = Self::find_runnable_system(&ready, dependencies, running) {
                let system: &mut SystemInfo<C> = systems[index].take().expect("Already executed");
                system.exec.exec(scope, res, context.clone(), running);

                start_count += 1;

                // okay, now that we started executing our system,
                // we remove the old one and add the ones which are
                // potentially ready

                let rem_pos = ready.iter().position(|x| *x == index).unwrap();
                ready.remove(rem_pos);

                for dependent in &system.dependents {
                    ready.push(*dependent);
                }
            }
        }
    }

    fn find_runnable_system(ready: &Vec<usize>,
                            dependencies: &Dependencies,
                            running: &AtomicBitSet)
                            -> Option<usize> {
        // Uh, this is probably
        // the worst code in
        // the history of Rust libraries

        'search: for &id in ready {
            for &dependency in &dependencies.dependencies[id] {
                if running.get(dependency) {
                    continue 'search;
                }
            }

            for &write in &dependencies.writes[id] {
                // A write is only allowed
                // if there are neither
                // writes nor reads.

                for &sys in &dependencies.rev_writes[&write] {
                    if sys != id && running.get(sys) {
                        continue 'search;
                    }
                }

                for &sys in &dependencies.rev_reads[&write] {
                    if sys != id && running.get(sys) {
                        continue 'search;
                    }
                }
            }

            for &read in &dependencies.reads[id] {
                // Unlimited reads can be performed
                // simultaneously, but no read is
                // allowed if there is also a write.

                for &sys in &dependencies.rev_writes[&read] {
                    if sys != id && running.get(sys) {
                        continue 'search;
                    }
                }
            }

            return Some(id);
        }

        None
    }
}

/// Builder for the [`Dispatcher`].
///
/// [`Dispatcher`]: struct.Dispatcher.html
///
/// # Examples
///
/// This is how you create a dispatcher with
/// a shared thread pool:
///
/// ```rust
/// # extern crate shred;
/// # #[macro_use]
/// # extern crate shred_derive;
/// # use shred::{DispatcherBuilder, Fetch, System, Resource};
/// # struct Res;
/// # impl Resource for Res {}
/// # #[derive(SystemData)] #[allow(unused)] struct Data<'a> { a: Fetch<'a, Res> }
/// # struct Dummy;
/// # impl<'a> System<'a> for Dummy {
/// #   type SystemData = Data<'a>;
/// #
/// #   fn work(&mut self, _: Data<'a>) {}
/// # }
/// #
/// # fn main() {
/// # let system_a = Dummy;
/// # let system_b = Dummy;
/// # let system_c = Dummy;
/// # let system_d = Dummy;
/// # let system_e = Dummy;
/// let dispatcher = DispatcherBuilder::new()
///     .add(system_a, "a", &[])
///     .add(system_b, "b", &["a"]) // b depends on a
///     .add(system_c, "c", &["a"]) // c also depends on a
///     .add(system_d, "d", &[])
///     .add(system_e, "e", &["c", "d"]) // e executes after c and d are finished
///     .finish();
/// # }
/// ```
///
#[derive(Default)]
pub struct DispatcherBuilder<'c, 't, C> {
    dependencies: Dependencies,
    ready: Vec<usize>,
    map: FnvHashMap<String, usize>,
    systems: Vec<SystemInfo<'c, 't, C>>,
    thread_pool: Option<Arc<ThreadPool>>,
}

impl<'c, 't, C> DispatcherBuilder<'c, 't, C>
    where C: 'c
{
    /// Creates a new `DispatcherBuilder` by
    /// using the `Default` implementation.
    ///
    /// The default behaviour is to create
    /// a thread pool on `finish`.
    /// If you already have a rayon `ThreadPool`,
    /// it's highly recommended to configure
    /// this builder to use it with `with_pool`
    /// instead.
    pub fn new() -> Self {
        DispatcherBuilder {
            dependencies: Default::default(),
            ready: Default::default(),
            map: Default::default(),
            systems: Default::default(),
            thread_pool: Default::default(),
        }
    }

    /// Adds a new system with a given name and a list of dependencies.
    /// Please not that the dependency should be added before
    /// you add the depending system.
    ///
    /// # Panics
    ///
    /// * if the specified dependency does not exist
    pub fn add<T>(mut self, system: T, name: &str, dep: &[&str]) -> Self
        where T: for<'a> System<'a, C> + Send + 't
    {
        let id = self.systems.len();
        let reads = unsafe { T::SystemData::reads() };
        let writes = unsafe { T::SystemData::writes() };

        let dependencies: Vec<usize> = dep.iter()
            .map(|x| {
                     *self.map
                          .get(x.to_owned())
                          .expect("No such system registered")
                 })
            .collect();

        for dependency in &dependencies {
            let dependency: &mut SystemInfo<C> = &mut self.systems[*dependency];
            dependency.dependents.push(id);
        }

        self.dependencies.add(id, reads, writes, dependencies);
        self.map.insert(name.to_owned(), id);

        if dep.is_empty() {
            self.ready.push(id);
        }

        let info = SystemInfo {
            dependents: Vec::new(),
            exec: Box::new(SystemDispatch::new(id, system)),
        };
        self.systems.push(info);

        self
    }

    /// Attach a rayon thread pool to the builder
    /// and use that instead of creating one.
    pub fn with_pool(mut self, pool: Arc<ThreadPool>) -> Self {
        self.thread_pool = Some(pool);

        self
    }

    /// Builds the `Dispatcher`.
    ///
    /// In the future, this method will
    /// precompute useful information in
    /// order to speed up dispatching.
    pub fn finish(self) -> Dispatcher<'c, 't, C> {
        let size = self.systems.len();

        Dispatcher {
            dependencies: self.dependencies,
            ready: self.ready,
            running: AtomicBitSet::with_size(size),
            systems: self.systems,
            thread_pool: self.thread_pool
                .unwrap_or_else(|| Self::create_thread_pool()),
        }
    }

    fn create_thread_pool() -> Arc<ThreadPool> {
        Arc::new(ThreadPool::new(Configuration::new().panic_handler(|x| {
            println!("Panic in worker thread: {:?}", x)
        }))
                         .expect("Invalid thread pool configuration"))
    }
}

trait ExecSystem<'c, C> {
    fn exec<'s>(&'s mut self, &Scope<'s>, &'s Resources, C, &'s AtomicBitSet) where 'c: 's;
}

struct SystemDispatch<T> {
    id: usize,
    system: T,
}

impl<T> SystemDispatch<T> {
    fn new(id: usize, system: T) -> Self {
        SystemDispatch {
            id: id,
            system: system,
        }
    }
}

impl<'c, C, T> ExecSystem<'c, C> for SystemDispatch<T>
    where C: 'c,
          T: for<'b> System<'b, C>
{
    fn exec<'s>(&'s mut self,
                scope: &Scope<'s>,
                res: &'s Resources,
                context: C,
                running: &'s AtomicBitSet)
        where 'c: 's
    {
        let data = T::SystemData::fetch(res);
        scope.spawn(move |_| {
                        self.system.work(data, context);
                        running.set(self.id, false)
                    })
    }
}

struct SystemInfo<'c, 't, C> {
    dependents: Vec<usize>,
    exec: Box<ExecSystem<'c, C> + Send + 't>,
}
