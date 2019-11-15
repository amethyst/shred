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

/// The `BatchController` is the additional trait that a normal System must
/// implement in order to be used as a system controlling the execution of a batch.
///
/// Note that the `System` must also implement `Send` because the `Dispatcher`
/// is by default un-send.
/// The safety of implementing `Send` is ensured by `BatchAccessor` which keeps
/// tracks of all used resources and thus the `System` can be safely executed in
/// with multiple threads.
pub trait BatchController<'a, 'b> {
    /// This associated type has to contain all resources batch controller uses directly.
    ///
    /// These have to be specified here, instead of `SystemData` (as
    /// a normal `System` does) because the sub `System`s can use the same `Resource`s
    /// of the `BatchController`.
    /// This make necessary to drop the references to the
    /// fetched `Resource`s in the batch controller before dispatching
    /// the sub `System`s.
    ///
    /// Now is easy to understand that specify the `BatchController`
    /// `Resource` in the `SystemData` doesn't allow to drop the reference
    /// before the sub dispatching; resulting in `Panic`.
    ///
    /// So this mechanism allows you to fetch safely the specified `Resource`
    /// in the `BatchController`.
    /// The example "examples/batch_dispatching.rs" show how to use it.
    ///
    /// Note that it's not required to specify the sub systems resources here
    /// because they are handled automatically.
    type BatchSystemData: SystemData<'a>;

    /// Creates an instance of the `BatchControllerSystem`.
    ///
    /// Usually this function is called internally by the `DispatcherBuilder`
    /// which creates the `BatchAccessor` correctly.
    /// The `Dispatcher` is constructed by the user elsewhere and passed to the
    /// `DispatcherBuilder` through the functions `with_batch` or `add_batch` and
    /// passed as argument to this function.
    ///
    /// This function is unsafe because an implementor of `BatchController` is expected
    /// to uphold quarantees of `Send` only when it's created with
    /// correctly constructed `BatchAccessor`.
    /// `BatchAccessor` is meant for tracking which resourced are being used by the controller.
    unsafe fn create(accessor: BatchAccessor, dispatcher: Dispatcher<'a, 'b>) -> Self;
}

/// The `DefaultBatchControllerSystem` is a simple implementation that will
/// dispatch the inner dispatcher one time.
///
/// Usually you want to create your own `Dispatcher`.
///
/// Is safe to implement `Send` and `Sync` because the `BatchAccessor` keep
/// tracks of all used resources and thus the `System` can be safely executed in
/// multi thread.
pub struct DefaultBatchControllerSystem<'a, 'b> {
    accessor: BatchAccessor,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> BatchController<'a, 'b> for DefaultBatchControllerSystem<'a, 'b> {
    type BatchSystemData = ();

    unsafe fn create(accessor: BatchAccessor, dispatcher: Dispatcher<'a, 'b>) -> Self {
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

/// Is safe to implement `Send` and `Sync` because the `BatchAccessor` keep
/// tracks of all used resources and thus the `System` can be safely executed in
/// multi thread.
unsafe impl<'a, 'b> Send for DefaultBatchControllerSystem<'a, 'b> {}

#[cfg(test)]
mod tests {

    use crate::{
        AccessorCow, BatchAccessor, BatchController, BatchUncheckedWorld, Dispatcher,
        DispatcherBuilder, RunningTime, System, World, Write,
    };

    /// This test demonstrate that the batch system is able to correctly setup
    /// its resources to default datas.
    #[test]
    fn test_setup() {
        let mut dispatcher = DispatcherBuilder::new()
            .with_batch::<CustomBatchControllerSystem>(
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
            .with_batch::<CustomBatchControllerSystem>(
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
            .with_batch::<CustomBatchControllerSystem>(
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

    pub struct CustomBatchControllerSystem<'a, 'b> {
        accessor: BatchAccessor,
        dispatcher: Dispatcher<'a, 'b>,
    }

    impl<'a, 'b> BatchController<'a, 'b> for CustomBatchControllerSystem<'a, 'b> {
        type BatchSystemData = ();

        unsafe fn create(accessor: BatchAccessor, dispatcher: Dispatcher<'a, 'b>) -> Self {
            CustomBatchControllerSystem {
                accessor,
                dispatcher,
            }
        }
    }

    impl<'a> System<'a> for CustomBatchControllerSystem<'_, '_> {
        type SystemData = BatchUncheckedWorld<'a>;

        fn run(&mut self, data: Self::SystemData) {
            for _i in 0..3 {
                self.dispatcher.dispatch(data.0);
            }
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

    unsafe impl<'a, 'b> Send for CustomBatchControllerSystem<'a, 'b> {}

}
