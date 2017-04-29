use fnv::FnvHashMap;
use rayon::Scope;

use {ResourceId, Task, TaskData};

pub struct Dispatcher {}

pub struct DispatcherBuilder {
    rev_reads: FnvHashMap<ResourceId, Vec<usize>>,
    rev_writes: FnvHashMap<ResourceId, Vec<usize>>,
    map: FnvHashMap<String, usize>,
    tasks: Vec<TaskInfo>,
}

impl DispatcherBuilder {
    pub fn new() -> Self {
        DispatcherBuilder {
            rev_reads: FnvHashMap::default(),
            rev_writes: FnvHashMap::default(),
            map: FnvHashMap::default(),
            tasks: Vec::new(),
        }
    }

    pub fn add<'a, T>(mut self, task: T, name: &str, dep: &[&str]) -> Self
        where T: Task,
              T::TaskData: TaskData<'a>
    {
        let id = self.tasks.len();
        let reads = unsafe { T::TaskData::reads() };
        let writes = unsafe { T::TaskData::writes() };

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

        let dependencies: Vec<usize> = dep.iter()
            .map(|x| {
                     *self.map
                         .get(x.to_owned())
                         .expect("No such task registered")
                 })
            .collect();

        self.map.insert(name.to_owned(), id);

        self
    }
}

struct TaskInfo {
    closure: Box<FnMut()>,
    dependencies: Vec<usize>,
    dependents: Vec<usize>,
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
}
