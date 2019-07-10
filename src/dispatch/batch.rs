use crate::{
    dispatch::Dispatcher, world::ResourceId, Accessor, AccessorCow, DynamicSystemData, RunningTime,
    System, SystemData, World,
};

/// The `BatchAccessor` is used to notify the main dispatcher of the read and
/// write resources of the `System`s contained by the `Batch` ("sub systems").
#[derive(Debug)]
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

/// The BatchController is the additional trait that a normal System must
/// implement in order to be used as BatchControllerSystem.
pub trait BatchController<'a, 'b> {
    /// Here you must set all the resource types that you want to use inside the
    /// `BatchControllerSystem`.
    /// All the system data needed for the controlled systems is fetched
    /// automatically.
    type BatchSystemData: SystemData<'a>;

    /// Create the instance of the BatchControllerSystem
    fn create(accessor: BatchAccessor, dispatcher: Dispatcher<'a, 'b>) -> Self;
}

/// The DefaultBatchControllerSystem is a simple implementation that will
/// dispatch the inner dispatcher one time.
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
