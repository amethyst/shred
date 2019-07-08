use crate::{
    dispatch::{Dispatcher, DispatcherBuilder},
    world::ResourceId,
    Accessor, AccessorCow, DynamicSystemData, RunningTime, System, SystemData, World,
};

/// The `BatchBuilder` is responsible for the creation of the Batch.
///
/// The `Batch` is a `System` which contains a `Dispatcher`.
/// By wrapping a `Dispatcher` inside a system, we can control the execution of a whole
/// group of system, without sacrificing parallelism or conciseness.
///
/// The BatchBuilder accept the dispatcher builder as parameter, and the type of the
/// System that will drive the execution of the internal dispatcher.
///
/// Note that depending on the dependencies of the SubSystems the Batch
/// can run in parallel with other Systems.
/// In addition the Sub Systems can run in parallel within the Batch.
#[derive(Default)]
pub struct BatchBuilder<'a, 'b> {
    dispatcher_builder: DispatcherBuilder<'a, 'b>,
}

impl<'a, 'b> BatchBuilder<'a, 'b> {
    /// Create a Batch using a dispatcher builder
    pub fn new(dispatcher_builder: DispatcherBuilder<'a, 'b>) -> Self {
        BatchBuilder { dispatcher_builder }
    }
}

impl<'a, 'b> BatchBuilder<'a, 'b> {
    /// Build the BatchSystem that will dispatch the inner dispatcher.
    ///
    /// Add the returned system to any dispatcher.
    pub fn build<T>(self) -> T
    where
        T: System<'a> + BatchController<'a, 'b> + Send + 'a,
    {
        let mut reads = self.dispatcher_builder.stages_builder.fetch_all_reads();
        reads.extend( <T::BatchSystemData as SystemData>::reads() );
        reads.sort();
        reads.dedup();

        let mut writes = self.dispatcher_builder.stages_builder.fetch_all_writes();
        writes.extend(<T::BatchSystemData as SystemData>::reads());
        writes.sort();
        writes.dedup();
        
        let accessor = BatchAccessor::new(reads, writes);
        let dispatcher = self.dispatcher_builder.build();

        T::create(accessor, dispatcher)
    }
}

/// The BatchAccessor is responsible for the notification of the read and write resources
/// of the subsystems
pub struct BatchAccessor {
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
}

impl BatchAccessor {
    /// Create the BatchAccessor
    pub fn new(reads: Vec<ResourceId>, writes: Vec<ResourceId>) -> Self {
        BatchAccessor { reads, writes }
    }
}

impl Accessor for BatchAccessor {
    fn try_new() -> Option<Self> {
        None
    }

    fn reads(&self) -> Vec<ResourceId> {
        self.reads.clone()
    }

    fn writes(&self) -> Vec<ResourceId> {
        self.writes.clone()
    }
}

/// The BatchUncheckedWorld wrap an instance of the world.
/// It's safe to be used only in the context of the Batch.
pub struct BatchUncheckedWorld<'a>(pub &'a World);

impl<'a> DynamicSystemData<'a> for BatchUncheckedWorld<'a> {
    type Accessor = BatchAccessor;

    fn setup(_accessor: &Self::Accessor, _world: &mut World) {}

    fn fetch(_access: &Self::Accessor, world: &'a World) -> Self {
        BatchUncheckedWorld(world)
    }
}

/// The BatchController is the additional trait that a normal System must implement
/// in order to be used as BatchControllerSystem.
pub trait BatchController<'a, 'b> {
    
    /// Here you must set all the Resources type that you want to use inside the
    /// BatchControllertSystem.
    /// All other are fetched automatically.
    type BatchSystemData: SystemData<'a>;
    
    /// Create the instance of the BatchControllerSystem
    fn create(accessor: BatchAccessor, dispatcher: Dispatcher<'a, 'b>) -> Self;

}

/// The DefaultBatchControllerSystem is a simple implementation that will dispatch
/// the inner dispatcher one time.
///
/// Usually you want to create your own dispatcher.
pub struct DefaultBatchControllerSystem<'a, 'b> {
    accessor: BatchAccessor,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> BatchController<'a, 'b> for DefaultBatchControllerSystem<'a, 'b> {
    type BatchSystemData = ();
    fn create(accessor: BatchAccessor, dispatcher: Dispatcher<'a, 'b>) -> Self {
        DefaultBatchControllerSystem {
            accessor,
            dispatcher,
        }
    }
}

impl<'a> System<'a> for DefaultBatchControllerSystem<'_, '_> {
    type SystemData = BatchUncheckedWorld<'a>;

    fn run(&mut self, data: Self::SystemData) {
        self.dispatcher.dispatch(data.0);
    }

    fn running_time(&self) -> RunningTime {
        RunningTime::VeryLong
    }

    fn accessor<'c>(&'c self) -> AccessorCow<'a, 'c, Self> {
        AccessorCow::Ref(&self.accessor)
    }

    fn setup(&mut self, world: &mut World) {
        self.dispatcher.setup(world);
    }
}

unsafe impl<'a, 'b> Send for DefaultBatchControllerSystem<'a, 'b> {}
