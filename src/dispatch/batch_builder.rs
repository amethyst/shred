use hashbrown::HashMap;

use crate::{
    dispatch::{
        dispatcher::SystemId,
        stage::{Stage, StagesBuilder},
    },
    system::RunningTime,
    world::ResourceId,
    RunNow, System, SystemData, World,
};

/// The BatchController is the additional trait that a normal System must implement
/// in order to be used as BatchController.
///
/// # Example
///
/// ```
/// use shred::{BatchController, System};
///
/// /// The following controller restart the execution of the Batch three times.
/// #[derive(Default)]
/// pub struct TestBatchControllerSystem{
///     iterations: i32,
/// }
///
/// impl BatchController for TestBatchControllerSystem{
///
///     fn prepare_dispatching(&mut self){
///         self.iterations = 0;
///     }
///     
///     fn want_to_dispatch(&mut self) -> bool{
///         self.iterations < 3
///     }
/// }
///
/// impl<'a> System<'a> for TestBatchControllerSystem {
///
///     type SystemData = ();
///
///     fn run(&mut self, (): Self::SystemData){
///
///         self.iterations += 1;
///     }
///
/// }
///
/// ```
///
/// Use the function `BatchBuilder::with_batch_controller` or `BatchBuilder::set_batch_controller`
/// to set your BatchControllerSystem.
pub trait BatchController {
    /// This function is called at the start of the BatchSystem execution.
    fn prepare_dispatching(&mut self);

    /// This control the internal dispatching of the sub systems.
    ///
    /// The following pseudo code show how it works.
    ///
    /// >loop{
    /// >    if batch_controller.want_to_dispatch() == false {
    /// >        break
    /// >    }
    /// >
    /// >    substep Dispatching
    /// >}
    ///
    fn want_to_dispatch(&mut self) -> bool;
}

pub trait BatchControllerSystem<'a>: BatchController + RunNow<'a> {}

impl<'a, T> BatchControllerSystem<'a> for T where T: BatchController + RunNow<'a> {}

/// This executes only 1 time the BatchSystem.
/// Use this controller doesn't make much
/// sense for this reason an error message will be print.
#[derive(Default)]
pub struct DefaultBatchControllerSystem {
    is_first_execution: bool,
}

impl BatchController for DefaultBatchControllerSystem {
    fn prepare_dispatching(&mut self) {
        self.is_first_execution = true;
    }

    fn want_to_dispatch(&mut self) -> bool {
        self.is_first_execution
    }
}

impl<'a> System<'a> for DefaultBatchControllerSystem {
    type SystemData = ();

    fn run(&mut self, (): Self::SystemData) {
        self.is_first_execution = false;
        eprintln!("The default batch controller is in use. Consider to use a normal System.")
    }
}

/// The BatchBuilder is responsible for the creation of the Batch.
/// It works similarly to the DispatcherBuilder, indeed is possible to set
/// the systems and the barriers.
/// The additional feature is that you can specify a BatchControllerSystem
/// to control the execution of the Batch.
///
/// Despite the similarities, it does a different thing since it create a Batch.
/// The Batch allow to define a group of Systems that can be dispatched
/// multiple times within the same main dispatching process.
/// For this reason the Batch dispatching is called SubDispatching.
///
/// Note that depending on the dependencies of the SubSystems the Batch
/// can run in parallel with other Systems.
/// In addition the Sub Systems can run in parallel within the Batch.
pub struct BatchBuilder<'a> {
    current_id: usize,
    map: HashMap<String, SystemId>,
    stages_builder: StagesBuilder<'a>,
    read_resources: Vec<ResourceId>,
    write_resources: Vec<ResourceId>,
    running_time: f32,
    system_count: u32,
    batch_controller: Box<for<'c> BatchControllerSystem<'c> + Send + 'a>,

    #[cfg(feature = "parallel")]
    thread_pool: Option<::std::sync::Arc<::rayon::ThreadPool>>,
}

impl<'a> Default for BatchBuilder<'a> {
    fn default() -> Self {
        BatchBuilder {
            current_id: 0,
            map: HashMap::default(),
            stages_builder: StagesBuilder::default(),
            read_resources: vec![],
            write_resources: vec![],
            running_time: 0.0,
            system_count: 0,
            batch_controller: Box::new(DefaultBatchControllerSystem::default()),
            thread_pool: None,
        }
    }
}

impl<'a> BatchBuilder<'a> {
    /// Creates a new `DispatcherBuilder` by using the `Default` implementation.
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the thread pool the systems will use
    #[cfg(feature = "parallel")]
    pub fn set_thread_pool(&mut self, thread_pool: Option<::std::sync::Arc<::rayon::ThreadPool>>) {
        self.thread_pool = thread_pool;
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
        use hashbrown::hash_map::Entry;

        let id = self.next_id();

        let dependencies = dep
            .iter()
            .map(|x| {
                *self
                    .map
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

        use crate::system::Accessor;

        let mut reads = system.accessor().reads() as Vec<ResourceId>;
        let mut writes = system.accessor().writes() as Vec<ResourceId>;

        self.read_resources.append(&mut reads);
        self.write_resources.append(&mut writes);

        self.running_time += match system.running_time() {
            RunningTime::VeryShort => 1.0,
            RunningTime::Short => 2.0,
            RunningTime::Average => 3.0,
            RunningTime::Long => 4.0,
            RunningTime::VeryLong => 5.0,
        };

        self.system_count += 1;

        self.stages_builder.insert(dependencies, id, system);
    }

    /// Set the batch controller system that is responsible for the execution of the Batch.
    pub fn with_batch_controller<T>(mut self, system: T) -> Self
    where
        T: for<'c> System<'c> + for<'c> BatchControllerSystem<'c> + Send + 'a,
    {
        self.set_batch_controller(system);

        self
    }

    /// Set the batch controller system that is responsible for the execution of the Batch.
    pub fn set_batch_controller<T>(&mut self, system: T)
    where
        T: for<'c> System<'c> + for<'c> BatchControllerSystem<'c> + Send + 'a,
    {
        use crate::system::Accessor;

        let mut reads = system.accessor().reads() as Vec<ResourceId>;
        let mut writes = system.accessor().writes() as Vec<ResourceId>;

        self.read_resources.append(&mut reads);
        self.write_resources.append(&mut writes);

        self.batch_controller = Box::new(system);
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
    /// Same as [DispatcherBuilder::add_barrier], but returns `self` to enable
    /// method chaining.
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

    fn next_id(&mut self) -> SystemId {
        let id = self.current_id;
        self.current_id += 1;

        SystemId(id)
    }

    /// Build the BatchSystem which take care to dispatch the subsystems.
    pub(crate) fn build(mut self) -> (Vec<ResourceId>, Vec<ResourceId>, BatchSystem<'a>) {
        self.read_resources.sort();
        self.read_resources.dedup();

        self.write_resources.sort();
        self.write_resources.dedup();

        let stage = self.stages_builder.build();

        // Average
        let batch_running_time = self.running_time / (self.system_count as f32);
        // Depending on the system count increase the running time
        let batch_running_time = batch_running_time + 2.0 * (self.system_count as f32 / 15.0);
        let batch_running_time = batch_running_time.round() as i32;

        let batch_running_time = match batch_running_time {
            1 => RunningTime::VeryShort,
            2 => RunningTime::Short,
            3 => RunningTime::Average,
            4 => RunningTime::Long,
            _ => RunningTime::VeryLong,
        };

        #[cfg(feature = "parallel")]
        let res = (
            self.read_resources,
            self.write_resources,
            BatchSystem::new(
                stage,
                self.batch_controller,
                batch_running_time,
                self.thread_pool.unwrap(),
            ),
        );

        #[cfg(not(feature = "parallel"))]
        let res = (
            self.read_resources,
            self.write_resources,
            Batch::new(stage, batch_running_time, batch_running_time),
        );

        res
    }
}

pub struct UncheckedWorld<'a>(pub &'a World);

impl<'a> SystemData<'a> for UncheckedWorld<'a> {
    fn setup(_world: &mut World) {}

    /// Returns the world
    fn fetch(res: &'a World) -> Self {
        UncheckedWorld(res)
    }

    /// Doesn't return nothing.
    /// To use this Struct is necessary check the dependecy manually
    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    /// Doesn't return nothing.
    /// To use this Struct is necessary check the dependecy manually
    fn writes() -> Vec<ResourceId> {
        vec![]
    }
}

/// The `BatchSystem` allow to register some subsystems that it will take
/// care to execute once the main dispatcher execute it.
/// It has the capability to restart its systems many times, and this
/// can be controlled by the user.
///
/// The BatchSystem keep track of its internal dependecies and is able to run
/// in parallel with other systems.
/// At the same time the subsystems can run in parallel within the BatchSystem.
///
/// To add a new batch system you have to call the function `with_batch` which
/// accept a `BatchBuilder` object that will take care to construct the `BatchSystem`.
pub struct BatchSystem<'a> {
    stages: Vec<Stage<'a>>,
    batch_controller: Box<for<'c> BatchControllerSystem<'c> + Send + 'a>,
    running_time: RunningTime,

    #[cfg(feature = "parallel")]
    thread_pool: ::std::sync::Arc<::rayon::ThreadPool>,
}

impl<'a> BatchSystem<'a> {
    #[cfg(feature = "parallel")]
    pub(crate) fn new(
        stages: Vec<Stage<'a>>,
        batch_controller: Box<for<'c> BatchControllerSystem<'c> + Send + 'a>,
        running_time: RunningTime,
        thread_pool: ::std::sync::Arc<::rayon::ThreadPool>,
    ) -> Self {
        BatchSystem {
            stages,
            batch_controller,
            running_time,
            thread_pool,
        }
    }

    #[cfg(not(feature = "parallel"))]
    pub(crate) fn new(
        stages: Vec<Stage<'a>>,
        batch_controller: Box<for<'c> BatchControllerSystem<'c> + Send + 'a>,
        running_time: RunningTime,
    ) -> Self {
        Batch {
            stages,
            batch_controller,
            running_time,
        }
    }

    /// Dispatches the systems (except thread local systems)
    /// in parallel given the resources to operate on.
    ///
    /// This operation blocks the
    /// executing thread.
    ///
    /// Only available with "parallel" feature enabled.
    ///
    /// Please note that this method assumes that no resource
    /// is currently borrowed. If that's the case, it panics.
    #[cfg(feature = "parallel")]
    pub fn dispatch_par(&mut self, world: &World) {
        let stages = &mut self.stages;

        self.thread_pool.install(move || {
            for stage in stages {
                stage.execute(world);
            }
        });
    }

    /// Dispatches the systems (except thread local systems) sequentially.
    ///
    /// This is useful if parallel overhead is
    /// too big or the platform does not support multithreading.
    ///
    /// Please note that this method assumes that no resource
    /// is currently borrowed. If that's the case, it panics.
    pub fn dispatch_seq(&mut self, world: &World) {
        for stage in &mut self.stages {
            stage.execute_seq(world);
        }
    }
}

impl<'a> System<'a> for BatchSystem<'_> {
    type SystemData = UncheckedWorld<'a>;

    fn run(&mut self, world: UncheckedWorld<'_>) {
        self.batch_controller.prepare_dispatching();
        loop {
            if self.batch_controller.want_to_dispatch() == false {
                break;
            }

            self.batch_controller.run_now(world.0);

            #[cfg(feature = "parallel")]
            self.dispatch_par(world.0);

            #[cfg(not(feature = "parallel"))]
            self.dispatch_seq(world.0);
        }
    }

    fn running_time(&self) -> RunningTime {
        self.running_time
    }

    fn setup(&mut self, world: &mut World) {
        self.batch_controller.setup(world);
        for stage in &mut self.stages {
            stage.setup(world);
        }
    }
}

unsafe impl Send for BatchSystem<'_> {}
unsafe impl Sync for BatchSystem<'_> {}
