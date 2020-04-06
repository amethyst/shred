use crate::{
    dispatch::Dispatcher, world::ResourceId, Accessor, AccessorCow, DynamicSystemData, RunningTime,
    System, SystemData, World,
};

/// The `BatchAccessor` is used to notify the main dispatcher of the read and
/// write resources of the `System`s contained in the batch ("sub systems").
#[derive(Debug)]
pub struct BatchAccessor {
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
}

impl BatchAccessor {
    /// Creates a `BatchAccessor`
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

/// The `BatchUncheckedWorld` wraps an instance of the world.
/// You have to specify this as `SystemData` for a `System` implementing `BatchController`.
pub struct BatchUncheckedWorld<'a>(pub &'a World);

impl<'a> DynamicSystemData<'a> for BatchUncheckedWorld<'a> {
    type Accessor = BatchAccessor;

    fn setup(_accessor: &Self::Accessor, _world: &mut World) {}

    fn fetch(_access: &Self::Accessor, world: &'a World) -> Self {
        BatchUncheckedWorld(world)
    }
}

/// The `BatchController` describes things that allow one to control how batches of systems are
/// executed.
///
/// A batch is a set of systems represented as a dispatcher (a sub-dispatcher, if you like).
///
/// It is registered with [`add_batch`][crate::DispatcherBuilder::add_batch], together with the
/// corresponding sub-dispatcher.
///
/// See the
/// [batch_dispatching](https://github.com/amethyst/shred/blob/master/examples/batch_dispatching.rs)
/// example.
///
/// The [`MultiDispatcher`] may help with implementing this in most common cases.
pub trait BatchController<'a, 'b, 'c> {
    /// This associated type has to contain all resources batch controller uses directly.
    ///
    /// Note that these are not fetched automatically for the controller, as is the case with
    /// ordinary [`System`]s. This is because the fetched references might need to be dropped
    /// before actually dispatching the other systems to avoid collisions on them and it would not
    /// be possible to perform using a parameter.
    ///
    /// Therefore, these are only *declared* here, but not automatically fetched. If the
    /// declaration does not match reality, the scheduler might make suboptimal decisions (if this
    /// declares more than is actually needed) or it may panic in runtime (in case it declares less
    /// and there happens to be a collision).
    type BatchSystemData: SystemData<'c>;

    /// The body of the controller.
    ///
    /// It is allowed to fetch (manually) and examine its
    /// [`BatchSystemData`][BatchController::BatchSystemData]. Then it shall drop all fetched
    /// references and is free to call `dispatcher.dispatch(world)` as many time as it sees fit.
    fn run(&mut self, world: &'c World, dispatcher: &mut Dispatcher<'a, 'b>);

    /// Estimate how heavy the whole controller, including the sub-systems, is in terms of
    /// computation costs.
    fn running_time(&self) -> RunningTime {
        RunningTime::VeryLong
    }
}

pub(crate) struct BatchControllerSystem<'a, 'b, C> {
    accessor: BatchAccessor,
    controller: C,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b, 'c, C> BatchControllerSystem<'a, 'b, C>
where
    C: BatchController<'a, 'b, 'c>,
{
    pub(crate) unsafe fn create(accessor: BatchAccessor, controller: C, dispatcher: Dispatcher<'a, 'b>)
        -> Self
    {
        Self {
            accessor,
            controller,
            dispatcher,
        }
    }
}

impl<'a, 'b, 'c, C> System<'c> for BatchControllerSystem<'a, 'b, C>
where
    C: BatchController<'a, 'b, 'c>,
{
    type SystemData = BatchUncheckedWorld<'c>;

    fn run(&mut self, data: Self::SystemData) {
        self.controller.run(data.0, &mut self.dispatcher);
    }

    fn running_time(&self) -> RunningTime {
        self.controller.running_time()
    }

    fn accessor<'s>(&'s self) -> AccessorCow<'c, 's, Self> {
        AccessorCow::Ref(&self.accessor)
    }

    fn setup(&mut self, world: &mut World) {
        world.setup::<C::BatchSystemData>();
        self.dispatcher.setup(world);
    }
}

unsafe impl<C: Send> Send for BatchControllerSystem<'_, '_, C> {}
unsafe impl<C: Sync> Sync for BatchControllerSystem<'_, '_, C> {}

/// The controlling parts of simplified [`BatchController`]s for running a batch fixed number of
/// times.
///
/// If one needs to implement a [`BatchController`] that first examines some data and decides
/// upfront how many times a set of sub-systems are to be dispatched, this can help with the
/// implementation. This is less flexible (it can't examine things in-between iterations of
/// dispatching, for example), but is often enough and more convenient as it avoids manual fetching
/// of the resources.
///
/// A common example is pausing a game ‒ based on some resource, the game physics systems are run
/// either 0 times or once.
///
/// A bigger example can be found in the
/// [multi_batch_dispatching](https://github.com/amethyst/shred/blob/master/examples/multi_batch_dispatching.rs).
///
/// To be useful, pass the controller to the constructor of [`MultiDispatcher`] and register with
/// [`add_batch`][crate::DispatcherBuilder::add_batch].
pub trait MultiDispatchController<'a>: Send {
    /// What data it needs to decide on how many times the subsystems should be run.
    ///
    /// This may overlap with system data used by the subsystems, but doesn't have to contain them.
    type SystemData: SystemData<'a>;

    /// Performs the decision.
    ///
    /// Returns the number of times the batch should be run and the [`MultiDispatcher`] will handle
    /// the actual execution.
    fn plan(&mut self, data: Self::SystemData) -> usize;
}

/// A bridge from [`MultiDispatchController`] to [`BatchController`].
///
/// This allows to turn a [`MultiDispatchController`] into a [`BatchController`] so it can be
/// registered with [`add_batch`][crate::DispatcherBuilder::add_batch].
pub struct MultiDispatcher<C> {
    controller: C,
}

impl<C> MultiDispatcher<C> {
    /// Constructor.
    ///
    /// The `controller` should implement [`MultiDispatchController`].
    pub fn new(controller: C) -> Self {
        Self {
            controller
        }
    }
}

impl<'a, 'b, 'c, C> BatchController<'a, 'b, 'c> for MultiDispatcher<C>
where
    C: MultiDispatchController<'c>,
{
    type BatchSystemData = C::SystemData;

    fn run(&mut self, world: &'c World, dispatcher: &mut Dispatcher<'a, 'b>) {
        let n = {
            let plan_data = world.system_data();
            self.controller.plan(plan_data)
        };

        for _ in 0..n {
            dispatcher.dispatch(world);
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{BatchController, Dispatcher, DispatcherBuilder, System, World, Write};

    /// This test demonstrate that the batch system is able to correctly setup
    /// its resources to default datas.
    #[test]
    fn test_setup() {
        let mut dispatcher = DispatcherBuilder::new()
            .with_batch(
                CustomBatchControllerSystem,
                DispatcherBuilder::new()
                    .with(BuyTomatoSystem, "buy_tomato_system", &[])
                    .with(BuyPotatoSystem, "buy_potato_system", &[]),
                "BatchSystemTest",
                &[],
            )
            .build();

        let mut world = World::empty();
        dispatcher.setup(&mut world);

        let potato_store = world.fetch::<PotatoStore>();
        let tomato_store = world.fetch::<TomatoStore>();
        assert!(!potato_store.is_store_open);
        assert!(!tomato_store.is_store_open);
        assert_eq!(potato_store.potato_count, 50);
        assert_eq!(tomato_store.tomato_count, 50);
    }

    /// This test demonstrate that the `CustomBatchControllerSystem` is able to
    /// dispatch its systems three times per dispatching in parallel.
    ///
    /// The parallel dispatching happen because there is no dependency between
    /// the two systems.
    ///
    /// Also the `OpenStoresSystem' and the `CloseStoresSystem` which request
    /// mutable access to the same dependencies used by the store systems
    /// are dispatched in sequence; respectivelly before and after the
    /// batch.
    ///
    /// Note that the Setup of the dispatcher is able to correctly create the
    /// store objects with default data.
    #[test]
    fn test_parallel_batch_execution() {
        let mut dispatcher = DispatcherBuilder::new()
            .with(OpenStoresSystem, "open_stores_system", &[])
            .with_batch(
                CustomBatchControllerSystem,
                DispatcherBuilder::new()
                    .with(BuyTomatoSystem, "buy_tomato_system", &[])
                    .with(BuyPotatoSystem, "buy_potato_system", &[]),
                "BatchSystemTest",
                &[],
            )
            .with(CloseStoresSystem, "close_stores_system", &[])
            .build();

        let mut world = World::empty();

        dispatcher.setup(&mut world);

        {
            // Initial assertion
            let potato_store = world.fetch::<PotatoStore>();
            let tomato_store = world.fetch::<TomatoStore>();
            assert!(!potato_store.is_store_open);
            assert!(!tomato_store.is_store_open);
            assert_eq!(potato_store.potato_count, 50);
            assert_eq!(tomato_store.tomato_count, 50);
        }

        // Running phase
        for _i in 0..10 {
            dispatcher.dispatch(&world);
        }

        {
            // This demonstrate that the batch system dispatch three times per
            // dispatch.
            let potato_store = world.fetch::<PotatoStore>();
            let tomato_store = world.fetch::<TomatoStore>();
            assert!(!potato_store.is_store_open);
            assert!(!tomato_store.is_store_open);
            assert_eq!(potato_store.potato_count, 50 - (1 * 3 * 10));
            assert_eq!(tomato_store.tomato_count, 50 - (1 * 3 * 10));
        }
    }

    /// This test demonstrate that the `CustomBatchControllerSystem` is able to
    /// dispatch its systems three times per dispatching in sequence.
    ///
    /// The sequence dispatching happen because there is a dependency between
    /// the two systems.
    ///
    /// Also the `OpenStoresSystem' and the `CloseStoresSystem` which request
    /// mutable access to the same dependencies used by the store systems
    /// are dispatched in sequence; respectivelly before and after the
    /// batch.
    ///
    /// The Setup of the dispatcher is able to correctly create the
    /// store objects with default data.
    /// Note the CustomWallet is created by the Batch setup demonstrating once
    /// again that it works.
    #[test]
    fn test_sequence_batch_execution() {
        let mut dispatcher = DispatcherBuilder::new()
            .with(OpenStoresSystem, "open_stores_system", &[])
            .with_batch(
                CustomBatchControllerSystem,
                DispatcherBuilder::new()
                    .with(BuyTomatoWalletSystem, "buy_tomato_system", &[])
                    .with(BuyPotatoWalletSystem, "buy_potato_system", &[]),
                "BatchSystemTest",
                &[],
            )
            .with(CloseStoresSystem, "close_stores_system", &[])
            .build();

        let mut world = World::empty();

        dispatcher.setup(&mut world);

        {
            // Initial assertion
            let potato_store = world.fetch::<PotatoStore>();
            let tomato_store = world.fetch::<TomatoStore>();
            let customer_wallet = world.fetch::<CustomerWallet>();
            assert!(!potato_store.is_store_open);
            assert!(!tomato_store.is_store_open);
            assert_eq!(potato_store.potato_count, 50);
            assert_eq!(tomato_store.tomato_count, 50);
            assert_eq!(customer_wallet.cents_count, 2000);
        }

        // Running phase
        for _i in 0..10 {
            dispatcher.dispatch(&world);
        }

        {
            // This demonstrate that the batch system dispatch three times per
            // dispatch.
            let potato_store = world.fetch::<PotatoStore>();
            let tomato_store = world.fetch::<TomatoStore>();
            let customer_wallet = world.fetch::<CustomerWallet>();
            assert!(!potato_store.is_store_open);
            assert!(!tomato_store.is_store_open);
            assert_eq!(potato_store.potato_count, 50 - (1 * 3 * 10));
            assert_eq!(tomato_store.tomato_count, 50 - (1 * 3 * 10));
            assert_eq!(customer_wallet.cents_count, 2000 - ((50 + 150) * 3 * 10));
        }
    }

    // Resources

    #[derive(Debug, Clone, Copy)]
    pub struct PotatoStore {
        pub is_store_open: bool,
        pub potato_count: i32,
    }

    impl Default for PotatoStore {
        fn default() -> Self {
            PotatoStore {
                is_store_open: false,
                potato_count: 50,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct TomatoStore {
        pub is_store_open: bool,
        pub tomato_count: i32,
    }

    impl Default for TomatoStore {
        fn default() -> Self {
            TomatoStore {
                is_store_open: false,
                tomato_count: 50,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct CustomerWallet {
        pub cents_count: i32,
    }

    impl Default for CustomerWallet {
        fn default() -> Self {
            CustomerWallet { cents_count: 2000 }
        }
    }

    // Open / Close Systems

    pub struct OpenStoresSystem;

    impl<'a> System<'a> for OpenStoresSystem {
        type SystemData = (Write<'a, PotatoStore>, Write<'a, TomatoStore>);

        fn run(&mut self, mut data: Self::SystemData) {
            data.0.is_store_open = true;
            data.1.is_store_open = true;
        }
    }

    pub struct CloseStoresSystem;

    impl<'a> System<'a> for CloseStoresSystem {
        type SystemData = (Write<'a, PotatoStore>, Write<'a, TomatoStore>);

        fn run(&mut self, mut data: Self::SystemData) {
            data.0.is_store_open = false;
            data.1.is_store_open = false;
        }
    }

    // Buy Systems

    pub struct BuyPotatoSystem;

    impl<'a> System<'a> for BuyPotatoSystem {
        type SystemData = Write<'a, PotatoStore>;

        fn run(&mut self, mut potato_store: Self::SystemData) {
            assert!(potato_store.is_store_open);
            potato_store.potato_count -= 1;
        }
    }

    pub struct BuyTomatoSystem;

    impl<'a> System<'a> for BuyTomatoSystem {
        type SystemData = Write<'a, TomatoStore>;

        fn run(&mut self, mut tomato_store: Self::SystemData) {
            assert!(tomato_store.is_store_open);
            tomato_store.tomato_count -= 1;
        }
    }

    // Buy systems with wallet

    pub struct BuyPotatoWalletSystem;

    impl<'a> System<'a> for BuyPotatoWalletSystem {
        type SystemData = (Write<'a, PotatoStore>, Write<'a, CustomerWallet>);

        fn run(&mut self, (mut potato_store, mut customer_wallet): Self::SystemData) {
            assert!(potato_store.is_store_open);
            potato_store.potato_count -= 1;
            customer_wallet.cents_count -= 50;
        }
    }

    pub struct BuyTomatoWalletSystem;

    impl<'a> System<'a> for BuyTomatoWalletSystem {
        type SystemData = (Write<'a, TomatoStore>, Write<'a, CustomerWallet>);

        fn run(&mut self, (mut tomato_store, mut customer_wallet): Self::SystemData) {
            assert!(tomato_store.is_store_open);
            tomato_store.tomato_count -= 1;
            customer_wallet.cents_count -= 150;
        }
    }

    // Custom Batch Controller which dispatch the systems three times

    pub struct CustomBatchControllerSystem;

    impl<'a, 'b> BatchController<'a, 'b, '_> for CustomBatchControllerSystem {
        type BatchSystemData = ();

        fn run(&mut self, world: &World, dispatcher: &mut Dispatcher<'a, 'b>) {
            for _i in 0..3 {
                dispatcher.dispatch(world);
            }
        }
    }
}
