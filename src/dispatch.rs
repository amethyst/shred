use std::sync::Arc;

use fnv::FnvHashMap;
use rayon::{Configuration, Scope, ThreadPool, scope};

use bitset::AtomicBitSet;
use {ResourceId, Resources, Task, TaskData};

#[derive(Default)]
pub struct Dependencies {
    dependencies: Vec<Vec<usize>>,
    rev_reads: FnvHashMap<ResourceId, Vec<usize>>,
    rev_writes: FnvHashMap<ResourceId, Vec<usize>>,
    reads: Vec<Vec<ResourceId>>,
    writes: Vec<Vec<ResourceId>>,
}

impl Dependencies {
    pub fn add(&mut self,
               id: usize,
               reads: Vec<ResourceId>,
               writes: Vec<ResourceId>,
               dependencies: Vec<usize>) {
        for read in &reads {
            self.rev_reads
                .entry(*read)
                .or_insert(Vec::new())
                .push(id);
        }

        for write in &writes {
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
/// tasks to be executed in parallel.
pub struct Dispatcher<'a> {
    dependencies: Dependencies,
    ready: Vec<usize>,
    running: AtomicBitSet,
    tasks: Vec<TaskInfo<'a>>,
    thread_pool: Arc<ThreadPool>,
}

impl<'a> Dispatcher<'a> {
    /// Dispatches the tasks given the
    /// resources to operate on.
    pub fn dispatch(&'a mut self, res: &'a mut Resources) {
        let dependencies = &self.dependencies;
        let ready = self.ready.clone();
        let running = &self.running;
        let tasks = &mut self.tasks;

        self.thread_pool
            .install(|| {
                scope(move |scope| {
                          Self::dispatch_inner(dependencies, ready, res, running, scope, tasks)
                      })
            });

        self.running.clear();
    }

    fn dispatch_inner<'s>(dependencies: &Dependencies,
                          mut ready: Vec<usize>,
                          res: &'a mut Resources,
                          running: &'a AtomicBitSet,
                          scope: &Scope<'s>,
                          tasks: &'a mut Vec<TaskInfo<'a>>)
        where 'a: 's
    {
        let mut start_count = 0;
        let num_tasks = tasks.len();
        let mut tasks: Vec<_> = tasks.iter_mut().map(|x| Some(x)).collect();

        while start_count < num_tasks {
            if let Some(index) = Self::find_runnable_task(&ready, dependencies, running) {
                let task: &mut TaskInfo = tasks[index].take().expect("Already executed");
                task.exec.exec(scope, res, running);

                start_count += 1;

                // okay, now that we started executing our task,
                // we remove the old one and add the ones which are
                // potentially ready

                let rem_pos = ready.iter().position(|x| *x == index).unwrap();
                ready.remove(rem_pos);

                for dependent in &task.dependents {
                    ready.push(*dependent);
                }
            }
        }
    }

    fn find_runnable_task(ready: &Vec<usize>,
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
#[derive(Default)]
pub struct DispatcherBuilder<'a> {
    dependencies: Dependencies,
    ready: Vec<usize>,
    map: FnvHashMap<String, usize>,
    tasks: Vec<TaskInfo<'a>>,
    thread_pool: Option<Arc<ThreadPool>>,
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
        DispatcherBuilder::default()
    }

    /// Adds a new task with a given name and a list of dependencies.
    /// Please not that the dependency should be added before
    /// you add the depending task.
    ///
    /// # Panics
    ///
    /// * if the specified dependency does not exist
    pub fn add<T>(mut self, task: T, name: &str, dep: &[&str]) -> Self
        where T: Task + Send + 'a,
              T::TaskData: TaskData<'a>
    {
        let id = self.tasks.len();
        let reads = unsafe { T::TaskData::reads() };
        let writes = unsafe { T::TaskData::writes() };

        let dependencies: Vec<usize> = dep.iter()
            .map(|x| {
                     *self.map
                          .get(x.to_owned())
                          .expect("No such task registered")
                 })
            .collect();

        for dependency in &dependencies {
            let dependency: &mut TaskInfo = &mut self.tasks[*dependency];
            dependency.dependents.push(id);
        }

        self.dependencies.add(id, reads, writes, dependencies);
        self.map.insert(name.to_owned(), id);

        if dep.is_empty() {
            self.ready.push(id);
        }

        let info = TaskInfo {
            dependents: Vec::new(),
            exec: Box::new(TaskDispatch::new(id, task)),
        };
        self.tasks.push(info);

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
        let size = self.tasks.len();

        Dispatcher {
            dependencies: self.dependencies,
            ready: self.ready,
            running: AtomicBitSet::with_size(size),
            tasks: self.tasks,
            thread_pool: self.thread_pool
                .unwrap_or_else(|| Self::create_thread_pool()),
        }
    }

    fn create_thread_pool() -> Arc<ThreadPool> {
        Arc::new(ThreadPool::new(
            Configuration::new()
                .panic_handler(|x| println!("Panic in worker thread: {:?}", x)))
            .expect("Invalid thread pool configuration"))
    }
}

trait ExecTask<'a> {
    fn exec<'s>(&'a mut self, &Scope<'s>, &'a Resources, &'a AtomicBitSet) where 'a: 's;
}

struct TaskDispatch<T> {
    id: usize,
    task: T,
}

impl<T> TaskDispatch<T> {
    fn new(id: usize, task: T) -> Self {
        TaskDispatch { id: id, task: task }
    }
}

impl<'a, T> ExecTask<'a> for TaskDispatch<T>
    where T: Task,
          T::TaskData: TaskData<'a>
{
    fn exec<'s>(&'a mut self, scope: &Scope<'s>, res: &'a Resources, running: &'a AtomicBitSet)
        where 'a: 's
    {
        let data = T::TaskData::fetch(res);
        scope.spawn(move |_| {
                        self.task.work(data);
                        running.set(self.id, false)
                    })
    }
}

struct TaskInfo<'a> {
    dependents: Vec<usize>,
    exec: Box<ExecTask<'a> + Send + 'a>,
}
