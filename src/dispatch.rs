use std::fmt::{Debug, Error as FormatError, Formatter};
#[cfg(not(target_os = "emscripten"))]
use std::sync::{Arc, Mutex};

use fnv::FnvHashMap;
#[cfg(not(target_os = "emscripten"))]
use pulse::Signal;
#[cfg(not(target_os = "emscripten"))]
use rayon_core::{Configuration, Scope, ThreadPool, scope};

#[cfg(not(target_os = "emscripten"))]
use bitset::AtomicBitSet;
use {ResourceId, Resources, System, SystemData};

#[cfg(not(target_os = "emscripten"))]
const ERR_NO_DISPATCH: &str = "wait() called before dispatch or called twice";

/// Like, `Dispatcher` but works
/// asynchronously.
#[cfg(not(target_os = "emscripten"))]
pub struct AsyncDispatcher {
    inner: Arc<Mutex<AsyncDispatcherInner>>,
    res: Arc<Resources>,
    signal: Option<Signal>,
    thread_local: Vec<Box<ExecSystem + 'static>>,
    thread_pool: Arc<ThreadPool>,
}

#[cfg(not(target_os = "emscripten"))]
impl AsyncDispatcher {
    /// Dispatches the systems asynchronously.
    ///
    /// If you want to wait for the systems to finish,
    /// call `wait()`.
    pub fn dispatch(&mut self) {
        let (signal, pulse) = Signal::new();
        self.signal = Some(signal);

        let inner = self.inner.clone();
        let res = self.res.clone();

        self.thread_pool
            .spawn_async(move || {
                {
                    let inner = inner;
                    let mut inner = inner.lock().unwrap();
                    let inner = &mut *inner;

                    Self::dispatch_inner(&inner.dependencies,
                                         inner.ready.clone(),
                                         res,
                                         &inner.running,
                                         &inner.stages,
                                         &mut inner.systems);
                }

                pulse.pulse();
            })
    }

    fn dispatch_inner(d: &Dependencies,
                      r: Vec<Vec<usize>>,
                      resources: Arc<Resources>,
                      run: &AtomicBitSet,
                      stages: &[usize],
                      sys: &mut Vec<SystemInfo>) {
        let res = &*resources;

        Dispatcher::dispatch_inner(d, r, res, run, stages, sys);
    }

    /// Waits for all the systems to finish
    /// and executes thread local systems (if there
    /// are any).
    pub fn wait(&mut self) {
        self.signal
            .take()
            .expect(ERR_NO_DISPATCH)
            .wait()
            .expect("The worker thread may have panicked");

        Dispatcher::dispatch_tl(&mut self.thread_local, &*self.res);
    }

    /// Returns the resources.
    ///
    /// If `wait()` wasn't called before,
    /// this method will do that.
    pub fn mut_res(&mut self) -> &mut Resources {
        if self.signal.is_some() {
            self.wait();
        }

        Arc::get_mut(&mut self.res).expect(ERR_NO_DISPATCH)
    }
}

#[cfg(not(target_os = "emscripten"))]
struct AsyncDispatcherInner {
    dependencies: Dependencies,
    ready: Vec<Vec<usize>>,
    running: AtomicBitSet,
    stages: Vec<usize>,
    systems: Vec<SystemInfo<'static>>,
}

#[derive(Debug, Default)]
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
                .or_insert_with(Vec::new)
                .push(id);

            self.rev_writes.entry(*read).or_insert_with(Vec::new);
        }

        for write in &writes {
            self.rev_reads.entry(*write).or_insert_with(Vec::new);

            self.rev_writes
                .entry(*write)
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.reads.push(reads);
        self.writes.push(writes);
        self.dependencies.push(dependencies);
    }
}

/// The dispatcher struct, allowing
/// systems to be executed in parallel.
pub struct Dispatcher<'t> {
    dependencies: Dependencies,
    #[cfg(not(target_os = "emscripten"))]
    ready: Vec<Vec<usize>>,
    #[cfg(not(target_os = "emscripten"))]
    running: AtomicBitSet,
    #[cfg(not(target_os = "emscripten"))]
    stages: Vec<usize>,
    systems: Vec<SystemInfo<'t>>,
    thread_local: Vec<Box<ExecSystem + 't>>,
    #[cfg(not(target_os = "emscripten"))]
    thread_pool: Arc<ThreadPool>,
}

impl<'t> Dispatcher<'t> {
    /// Dispatch systems with given resources and context.
    ///
    /// This function automatically redirects to
    ///
    /// * [`dispatch_par`] in case it is supported
    /// * [`dispatch_seq`] otherwise
    ///
    /// [`dispatch_par`]: struct.Dispatcher.html#method.dispatch_par
    /// [`dispatch_seq`]: struct.Dispatcher.html#method.dispatch_seq
    pub fn dispatch(&mut self, res: &mut Resources) {
        #[cfg(not(target_os = "emscripten"))]
        self.dispatch_par(res);

        #[cfg(target_os = "emscripten")]
        self.dispatch_seq(res);
    }

    /// Dispatches the systems in parallel given the
    /// resources to operate on.
    ///
    /// This operation blocks the
    /// executing thread.
    ///
    /// Only available on platforms with
    /// multithreading support (so not on emscripten).
    #[cfg(not(target_os = "emscripten"))]
    pub fn dispatch_par(&mut self, res: &mut Resources) {
        let d = &self.dependencies;
        let ready = self.ready.clone();
        let res = res as &Resources;
        let running = &self.running;
        let stages = &self.stages;
        let systems = &mut self.systems;

        self.thread_pool
            .install(move || Self::dispatch_inner(d, ready, res, running, stages, systems));

        self.running.clear();

        Self::dispatch_tl(&mut self.thread_local, res);
    }

    /// Dispatches all systems sequentially.
    ///
    /// This is useful if parallel overhead is
    /// too big or the platform does not support multithreading.
    pub fn dispatch_seq(&mut self, res: &mut Resources) {
        for system in &mut self.systems {
            system.exec.exec_seq(res);
        }

        Self::dispatch_tl(&mut self.thread_local, res);
    }

    fn dispatch_tl(tl: &mut [Box<ExecSystem + 't>], res: &Resources) {
        for tl in tl {
            tl.exec_seq(res);
        }
    }

    #[cfg(not(target_os = "emscripten"))]
    fn dispatch_inner<'s>(dependencies: &Dependencies,
                          mut ready: Vec<Vec<usize>>,
                          res: &'s Resources,
                          running: &'s AtomicBitSet,
                          stages: &[usize],
                          systems: &'s mut Vec<SystemInfo>) {
        let mut systems: Vec<_> = systems.iter_mut().map(Some).collect();

        for (num_systems, ready) in stages.iter().zip(ready.iter_mut()) {
            let num_systems = *num_systems;

            let systems = &mut systems;

            scope(move |scope| {
                let mut start_count = 0;

                while start_count < num_systems {
                    if let Some(index) = Self::find_runnable_system(&ready, dependencies, running) {
                        let system: &mut SystemInfo =
                            systems[index].take().expect("Already executed");
                        system.exec.exec(scope, res, running);

                        start_count += 1;

                        // okay, now that we started executing our system,
                        // we remove the old one and add the ones which are
                        // potentially ready

                        let rem_pos = ready.iter().position(|x| *x == index).unwrap();
                        ready.remove(rem_pos);

                        for dependent in &system.dependents {
                            ready.push(*dependent);
                        }
                    } else {
                        use std::thread;
                        use std::time::Duration;

                        thread::sleep(Duration::new(0, 10));
                    }
                }
            });
        }
    }

    #[cfg(not(target_os = "emscripten"))]
    fn find_runnable_system(ready: &[usize],
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

impl<'t> Debug for Dispatcher<'t> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FormatError> {
        f.debug_struct("Dispatcher")
            .field("dependencies", &self.dependencies)
            .field("ready", &self.ready)
            .finish()
    }
}

/// Builder for the [`Dispatcher`].
///
/// [`Dispatcher`]: struct.Dispatcher.html
///
/// ## Barriers
///
/// Barriers are a way of sequentializing parts of
/// the system execution. See `add_barrier()`.
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
/// # #[derive(Debug)] struct Res;
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
///     .add(system_a, "a", &[])
///     .add(system_b, "b", &["a"]) // b depends on a
///     .add(system_c, "c", &["a"]) // c also depends on a
///     .add(system_d, "d", &[])
///     .add(system_e, "e", &["c", "d"]) // e executes after c and d are finished
///     .build();
/// # }
/// ```
///
pub struct DispatcherBuilder<'t> {
    dependencies: Dependencies,
    map: FnvHashMap<String, usize>,
    #[cfg(not(target_os = "emscripten"))]
    ready: Vec<Vec<usize>>,
    systems: Vec<SystemInfo<'t>>,
    #[cfg(not(target_os = "emscripten"))]
    stages: Vec<usize>,
    thread_local: Vec<Box<ExecSystem + 't>>,
    #[cfg(not(target_os = "emscripten"))]
    thread_pool: Option<Arc<ThreadPool>>,
    #[cfg(not(target_os = "emscripten"))]
    unstaged: usize,
    #[cfg(not(target_os = "emscripten"))]
    unstaged_ready: Vec<usize>,
}

impl<'t> DispatcherBuilder<'t> {
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
        Default::default()
    }

    /// Adds a new system with a given name and a list of dependencies.
    /// Please not that the dependency should be added before
    /// you add the depending system.
    ///
    /// # Panics
    ///
    /// * if the specified dependency does not exist
    pub fn add<T>(self, system: T, name: &str, dep: &[&str]) -> Self
        where T: for<'a> System<'a> + Send + 't
    {
        self.add_with_id(system, 0, name, dep)
    }

    /// Adds a new system with a given name, a data id and a list of dependencies.
    /// Please not that the dependency should be added before
    /// you add the depending system.
    ///
    /// # Panics
    ///
    /// * if the specified dependency does not exist
    pub fn add_with_id<T>(mut self, system: T, data_id: usize, name: &str, dep: &[&str]) -> Self
        where T: for<'a> System<'a> + Send + 't
    {
        let id = self.systems.len();
        let reads = unsafe { T::SystemData::reads(data_id) };
        let writes = unsafe { T::SystemData::writes(data_id) };

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
            self.unstaged_ready.push(id);
        }

        #[cfg(not(target_os = "emscripten"))]
        let exec = SystemDispatch::new(id, data_id, system);

        #[cfg(target_os = "emscripten")]
        let exec = SystemDispatch::new(system, data_id);

        let info = SystemInfo {
            dependents: Vec::new(),
            exec: Box::new(exec),
        };
        self.systems.push(info);

        self.unstaged += 1;

        self
    }

    /// Adds a new thread local system.
    ///
    /// Please only use this if your struct is
    /// not `Send` and `Sync`.
    ///
    /// Thread-local systems are dispatched
    /// in-order.
    pub fn add_thread_local<T>(self, system: T) -> Self
        where T: for<'a> System<'a> + 't
    {
        self.add_thread_local_with_id(system, 0)
    }

    /// Adds a new thread local system with a given data id.
    ///
    /// Please only use this if your struct is
    /// not `Send` and `Sync`.
    ///
    /// Thread-local systems are dispatched
    /// in-order.
    pub fn add_thread_local_with_id<T>(mut self, system: T, data_id: usize) -> Self
        where T: for<'a> System<'a> + 't
    {

        #[cfg(not(target_os = "emscripten"))]
        self.thread_local
            .push(Box::new(SystemDispatch::new(0, data_id, system)));

        #[cfg(target_os = "emscripten")]
        self.thread_local
            .push(Box::new(SystemDispatch::new(system, data_id)));

        self
    }

    /// Inserts a barrier which assures that all systems
    /// added before the barrier are executed before the ones
    /// after this barrier.
    ///
    /// Does nothing if there were no systems added
    /// since the last call to `add_barrier()`.
    ///
    /// Thread-local systems are not affected by barriers;
    /// they're always executed at the end.
    pub fn add_barrier(mut self) -> Self {
        self.stage_unstaged();

        self
    }

    /// Attach a rayon thread pool to the builder
    /// and use that instead of creating one.
    #[cfg(not(target_os = "emscripten"))]
    pub fn with_pool(mut self, pool: Arc<ThreadPool>) -> Self {
        self.thread_pool = Some(pool);

        self
    }

    /// Builds the `Dispatcher`.
    ///
    /// In the future, this method will
    /// precompute useful information in
    /// order to speed up dispatching.
    pub fn build(mut self) -> Dispatcher<'t> {
        self.stage_unstaged();

        #[cfg(not(target_os = "emscripten"))]
        let size = self.systems.len();

        #[cfg(not(target_os = "emscripten"))]
        let d = Dispatcher {
            dependencies: self.dependencies,
            ready: self.ready,
            running: AtomicBitSet::with_size(size),
            stages: self.stages,
            systems: self.systems,
            thread_local: self.thread_local,
            thread_pool: self.thread_pool.unwrap_or_else(Self::create_thread_pool),
        };

        #[cfg(target_os = "emscripten")]
        let d = Dispatcher {
            dependencies: self.dependencies,
            ready: self.ready,
            systems: self.systems,
            thread_local: self.thread_local,
        };

        d
    }

    #[cfg(not(target_os = "emscripten"))]
    fn create_thread_pool() -> Arc<ThreadPool> {
        Arc::new(ThreadPool::new(Configuration::new().panic_handler(|x| {
            println!("Panic in worker thread: {:?}", x)
        }))
                         .expect("Invalid thread pool configuration"))
    }

    #[cfg(not(target_os = "emscripten"))]
    fn stage_unstaged(&mut self) {
        use std::mem::swap;

        if self.unstaged == 0 {
            return;
        }

        let mut unstaged_ready = Vec::new();
        swap(&mut unstaged_ready, &mut self.unstaged_ready);
        self.ready.push(unstaged_ready);
        self.stages.push(self.unstaged);
        self.unstaged = 0;
    }

    #[allow(unused)]
    #[cfg(target_os = "emscripten")]
    fn stage_unstaged(&mut self) {}
}

#[cfg(not(target_os = "emscripten"))]
impl DispatcherBuilder<'static> {
    /// Builds an async dispatcher.
    ///
    /// It does not allow non-static types and
    /// accepts a `Resource` struct.
    pub fn build_async(mut self, res: Resources) -> AsyncDispatcher {
        self.stage_unstaged();

        let size = self.systems.len();

        let inner = AsyncDispatcherInner {
            dependencies: self.dependencies,
            ready: self.ready,
            running: AtomicBitSet::with_size(size),
            stages: self.stages,
            systems: self.systems,
        };

        AsyncDispatcher {
            inner: Arc::new(Mutex::new(inner)),
            res: Arc::new(res),
            signal: None,
            thread_local: self.thread_local,
            thread_pool: self.thread_pool.unwrap_or_else(Self::create_thread_pool),
        }
    }
}

impl<'t> Debug for DispatcherBuilder<'t> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FormatError> {
        f.debug_struct("DispatcherBuilder")
            .field("dependencies", &self.dependencies)
            .field("map", &self.map)
            .finish()
    }
}

impl<'t> Default for DispatcherBuilder<'t> {
    fn default() -> Self {
        #[cfg(not(target_os = "emscripten"))]
        let d = DispatcherBuilder {
            dependencies: Default::default(),
            ready: Default::default(),
            map: Default::default(),
            stages: Default::default(),
            systems: Default::default(),
            thread_local: Default::default(),
            thread_pool: Default::default(),
            unstaged: Default::default(),
            unstaged_ready: Default::default(),
        };

        #[cfg(target_os = "emscripten")]
        let d = DispatcherBuilder {
            dependencies: Default::default(),
            map: Default::default(),
            systems: Default::default(),
            thread_local: Default::default(),
        };

        d
    }
}

trait ExecSystem {
    #[cfg(not(target_os = "emscripten"))]
    fn exec<'s>(&'s mut self, s: &Scope<'s>, res: &'s Resources, running: &'s AtomicBitSet);

    fn exec_seq(&mut self, res: &Resources);
}

struct SystemDispatch<T> {
    #[cfg(not(target_os = "emscripten"))]
    id: usize,
    data_id: usize,
    system: T,
}

impl<T> SystemDispatch<T> {
    #[cfg(not(target_os = "emscripten"))]
    fn new(id: usize, data_id: usize, system: T) -> Self {
        SystemDispatch {
            id: id,
            data_id: data_id,
            system: system,
        }
    }

    #[cfg(target_os = "emscripten")]
    fn new(system: T, data_id: usize) -> Self {
        SystemDispatch {
            system: system,
            data_id: data_id,
        }
    }
}

impl<T> ExecSystem for SystemDispatch<T>
    where T: for<'b> System<'b>
{
    #[cfg(not(target_os = "emscripten"))]
    fn exec<'s>(&'s mut self, scope: &Scope<'s>, res: &'s Resources, running: &'s AtomicBitSet) {
        running.set(self.id, true);
        let data = T::SystemData::fetch(res, self.data_id);
        scope.spawn(move |_| {
                        self.system.run(data);
                        running.set(self.id, false)
                    })
    }

    fn exec_seq(&mut self, res: &Resources) {
        run_now(&mut self.system, res, self.data_id);
    }
}

struct SystemInfo<'t> {
    dependents: Vec<usize>,
    exec: Box<ExecSystem + Send + 't>,
}

/// Runs a system right now.
///
/// You usually want to use the [`Dispatcher`]
/// instead.
///
/// [`Dispatcher`]: struct.Dispatcher.html
pub fn run_now<'a, T>(sys: &mut T, res: &'a Resources, id: usize)
    where T: System<'a>
{
    let data = T::SystemData::fetch(res, id);
    sys.run(data);
}
