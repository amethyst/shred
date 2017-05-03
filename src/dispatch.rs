use std::sync::Arc;

use fnv::FnvHashMap;
use rayon::{Configuration, Scope, ThreadPool};

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

pub struct Dispatcher<'r, 't> {
    dependencies: Dependencies,
    fulfilled: Vec<usize>,
    running: Arc<AtomicBitSet>,
    tasks: Vec<TaskInfo<'r, 't>>,
    thread_pool: Arc<ThreadPool>,
}

impl<'r, 't> Dispatcher<'r, 't> {
    pub fn dispatch(&mut self, res: &'r mut Resources) {}
}

#[derive(Default)]
pub struct DispatcherBuilder<'r, 't> {
    dependencies: Dependencies,
    fulfilled: Vec<usize>,
    map: FnvHashMap<String, usize>,
    tasks: Vec<TaskInfo<'r, 't>>,
    thread_pool: Option<Arc<ThreadPool>>,
}

impl<'r, 't> DispatcherBuilder<'r, 't> {
    pub fn new() -> Self {
        DispatcherBuilder::default()
    }

    pub fn add<T>(mut self, mut task: T, name: &str, dep: &[&str]) -> Self
        where T: Task + 't,
              T::TaskData: TaskData<'r>
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
            self.fulfilled.push(id);
        }

        let info = TaskInfo {
            dependents: Vec::new(),
            exec: Box::new(TaskDispatch::new(id, task)) as Box<ExecTask>,
        };
        self.tasks.push(info);

        self
    }

    pub fn with_pool(mut self, pool: Arc<ThreadPool>) -> Self {
        self.thread_pool = Some(pool);

        self
    }

    pub fn finish(self) -> Dispatcher<'r, 't> {
        let size = self.tasks.len();

        Dispatcher {
            dependencies: self.dependencies,
            fulfilled: self.fulfilled,
            running: Arc::new(AtomicBitSet::with_size(size)),
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

trait ExecTask<'r> {
    fn exec<'b, 's, 'a>(&'b mut self, &'s Scope<'s>, &'r Resources, &'a AtomicBitSet)
        where 'a: 's,
              'b: 's;
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

impl<'r, T> ExecTask<'r> for TaskDispatch<T>
    where T: Task,
          T::TaskData: TaskData<'r>
{
    fn exec<'b, 's, 'a>(&'b mut self,
                        scope: &'s Scope<'s>,
                        res: &'r Resources,
                        running: &'a AtomicBitSet)
        where 'a: 's,
              'b: 's
    {
        let data = T::TaskData::fetch(res);
        scope.spawn(move |_| {
                        self.task.work(data);
                        running.set(self.id, false)
                    })
    }
}

struct TaskInfo<'r, 't> {
    dependents: Vec<usize>,
    exec: Box<ExecTask<'r> + 't>,
}
