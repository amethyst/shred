use std::sync::Arc;

use fnv::FnvHashMap;
use rayon::{Configuration, ThreadPool, scope};

use bitset::AtomicBitSet;
use {ResourceId, Resources, SystemData};
use system::Helper;

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
            self.rev_reads
                .entry(*read)
                .or_insert(Vec::new())
                .push(id);

            self.rev_writes.entry(*read).or_insert(Vec::new());
        }

        for write in &writes {
            self.rev_reads.entry(*write).or_insert(Vec::new());

            self.rev_writes
                .entry(*write)
                .or_insert(Vec::new())
                .push(id);
        }

        self.reads.push(reads);
        self.writes.push(writes);
        self.dependencies.push(dependencies);
    }
}

/// The dispatcher struct, allowing
/// systems to be executed in parallel.
pub struct Dispatcher<'a> {
    dependencies: Dependencies,
    ready: Vec<usize>,
    running: AtomicBitSet,
    systems: Vec<SystemInfo>,
    thread_pool: Arc<ThreadPool>,
    system_workers: Vec<SystemWorker<'a>>,
}

impl<'a> Dispatcher<'a> {
    /// Dispatches the systems given the
    /// resources to operate on.
    ///
    /// This operation blocks the
    /// executing thread.
    pub fn dispatch<'b: 'a>(&mut self, res: &'b Resources) {
        let dependencies = &self.dependencies;
        let mut ready = self.ready.clone();
        let running = &self.running;
        let systems = &mut self.systems;
        let workers = &mut self.system_workers;

        self.thread_pool
            .install(|| {
                scope(move |scope| {
                    let mut start_count = 0;
                    let num_systems = systems.len();
                    let mut systems: Vec<_> = systems.iter_mut().map(|x| Some(x)).collect();
                    let mut workers: Vec<_> = workers.iter_mut().map(|x| Some(x)).collect();

                    while start_count < num_systems {
                        if let Some(index) = Self::find_runnable_system(&ready, dependencies, running) {
                            let system: &mut SystemInfo = systems[index].take().expect("Already executed");
                            let worker = workers[index].take().expect("Already executed");
                            let data = (worker.fetch)(res);
                            scope.spawn(move |_| (worker.exec)(data, running) );

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
                })
            });

        self.running.clear();
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
/// # use shred::{DispatcherBuilder, Fetch, Resource};
/// # struct Res;
/// # impl Resource for Res {}
/// # #[derive(SystemData)] #[allow(unused)] struct Data<'a> { a: Fetch<'a, Res> }
/// # struct Dummy;
/// # fn work<'a>(_: &mut Dummy, _: Data<'a>) {}
/// #
/// # fn main() {
/// # let system_a = Dummy;
/// # let system_b = Dummy;
/// # let system_c = Dummy;
/// # let system_d = Dummy;
/// # let system_e = Dummy;
/// let dispatcher = DispatcherBuilder::new()
///     .add("a", work, system_a, &[])
///     .add("b", work, system_b, &["a"]) // b depends on a
///     .add("c", work, system_c, &["a"]) // c also depends on a
///     .add("d", work, system_d, &[])
///     .add("e", work, system_e, &["c", "d"]) // e executes after c and d are finished
///     .finish();
/// # }
/// ```
///
pub struct DispatcherBuilder<'a> {
    dependencies: Dependencies,
    ready: Vec<usize>,
    map: FnvHashMap<String, usize>,
    systems: Vec<SystemInfo>,
    thread_pool: Option<Arc<ThreadPool>>,
    system_workers: Vec<SystemWorker<'a>>,
}

impl<'a> DispatcherBuilder<'a> {
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
            dependencies: Dependencies::default(),
            ready: Vec::default(),
            map: FnvHashMap::default(),
            systems: Vec::default(),
            thread_pool: None,
            system_workers: Vec::default(),
        }
    }

    /// Adds a new system with a given name and a list of dependencies.
    /// Please not that the dependency should be added before
    /// you add the depending system.
    ///
    /// # Panics
    ///
    /// * if the specified dependency does not exist
    pub fn add<T, F, S>(mut self, name: &str, func: F, mut system: S, dep: &[&str]) -> Self
        where T: SystemData<'a> + 'a,
              F: Fn(&mut S, T) + Send + Sync + 'static,
              S: Send + Sync + 'static
    {
        let id = self.systems.len();
        let reads = unsafe { T::reads() };
        let writes = unsafe { T::writes() };

        let dependencies: Vec<usize> = dep.iter()
            .map(|x| {
                     *self.map
                          .get(x.to_owned())
                          .expect("No such system registered")
                 })
            .collect();

        for dependency in &dependencies {
            let dependency: &mut SystemInfo = &mut self.systems[*dependency];
            dependency.dependents.push(id);
        }

        self.dependencies.add(id, reads, writes, dependencies);
        self.map.insert(name.to_owned(), id);

        if dep.is_empty() {
            self.ready.push(id);
        }

        let info = SystemInfo { dependents: Vec::new() };
        self.systems.push(info);

        let worker = SystemWorker {
            exec: Box::new(move |data: Box<Helper + 'a>, running: &AtomicBitSet| {
                    let raw: *mut Helper = Box::into_raw(data);
                    let data = unsafe { Box::from_raw(raw as *mut T) };
                    func(&mut system, *data);
                    running.set(id, false);
            }),
            fetch: Box::new(move |res: &'a Resources| Box::new(T::fetch(res))),
        };
        self.system_workers.push(worker);

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
    pub fn finish(self) -> Dispatcher<'a> {
        let size = self.systems.len();

        Dispatcher {
            dependencies: self.dependencies,
            ready: self.ready,
            running: AtomicBitSet::with_size(size),
            systems: self.systems,
            thread_pool: self.thread_pool
                .unwrap_or_else(|| Self::create_thread_pool()),
            system_workers: self.system_workers,
        }
    }

    fn create_thread_pool() -> Arc<ThreadPool> {
        Arc::new(ThreadPool::new(
            Configuration::new()
                .panic_handler(|x| println!("Panic in worker thread: {:?}", x)))
            .expect("Invalid thread pool configuration"))
    }
}

struct SystemInfo {
    dependents: Vec<usize>,
}

struct SystemWorker<'a> {
    exec: Box<FnMut(Box<Helper + 'a>, &AtomicBitSet) + Send + Sync>,
    fetch: Box<Fn(&'a Resources) -> Box<Helper + 'a> + Send + Sync>,
}