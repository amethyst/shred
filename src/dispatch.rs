use std::sync::Arc;

use fnv::FnvHashMap;
use rayon::Scope;

use bitset::AtomicBitSet;
use {ResourceId, Task, TaskData};

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

pub struct Dispatcher {
    dependencies: Dependencies,
    fulfilled: Vec<usize>,
    running: Arc<AtomicBitSet>,
    tasks: Vec<TaskInfo>,
}

#[derive(Default)]
pub struct DispatcherBuilder {
    dependencies: Dependencies,
    fulfilled: Vec<usize>,
    map: FnvHashMap<String, usize>,
    tasks: Vec<TaskInfo>,
}

impl DispatcherBuilder {
    pub fn new() -> Self {
        DispatcherBuilder::default()
    }

    pub fn add<'a, T>(mut self, task: T, name: &str, dep: &[&str]) -> Self
        where T: Task,
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
            self.fulfilled.push(id);
        }

        self.tasks.push(TaskInfo {
            closure: Box::new(|| { unimplemented!() }),
            dependents: Vec::new(),
        });

        self
    }

    pub fn finish(self) -> Dispatcher {
        let size = self.tasks.len();

        Dispatcher {
            dependencies: self.dependencies,
            fulfilled: self.fulfilled,
            running: Arc::new(AtomicBitSet::with_size(size)),
            tasks: self.tasks,
        }
    }
}

struct TaskInfo {
    closure: Box<FnMut()>,
    dependents: Vec<usize>,
}
