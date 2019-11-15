//! This example shows how to use and define a batch dispatcher.
//!
//! The batch feature allows to control the dispatching of a group of
//! systems.
//!
//! Specifically here we have three Systems
//! - `SayHelloSystem`: Which is directly registered under the main
//!   dispatcher.
//! - `BuyTomatoSystem` and `BuyPotatoSystem` are registered to the batch.
//!
//! Notice that none of these systems are directly depending on others.
//! The `SayHelloSystem` is requesting the resources `TomatoStore` and
//! `PotatoStore`, which are also requested by the other two systems inside
//! the batch and by the batch controller itself.
//!
//! This example demonstrates that the batch dispatcher is able to affect on how the systems inside the batch are executed
//!
//! This is done by defining `CustomBatchControllerSystem` which executes its inner `System`s
//! three times.

use shred::{
    AccessorCow, BatchAccessor, BatchController, BatchUncheckedWorld, Dispatcher,
    DispatcherBuilder, Read, RunningTime, System, World, Write,
};
use std::{thread::sleep, time::Duration};

fn main() {
    let mut dispatcher = DispatcherBuilder::new()
        .with(SayHelloSystem, "say_hello_system", &[])
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

    // Running phase
    for i in 0..10 {
        println!("Dispatching {} ", i);

        dispatcher.dispatch(&world);
        sleep(Duration::new(0, 100000000));
    }

    // Done
    println!("Execution finished");
}

// Resources

#[derive(Default)]
pub struct PotatoStore(i32);

#[derive(Default)]
pub struct TomatoStore(f32);

/// System that says "Hello!"

pub struct SayHelloSystem;

impl<'a> System<'a> for SayHelloSystem {
    type SystemData = (Write<'a, PotatoStore>, Write<'a, TomatoStore>);

    fn run(&mut self, _data: Self::SystemData) {
        println!("Hello!")
    }
}

/// System that says "Buy Potato"

pub struct BuyPotatoSystem;

impl<'a> System<'a> for BuyPotatoSystem {
    type SystemData = Write<'a, PotatoStore>;

    fn run(&mut self, _data: Self::SystemData) {
        println!("Buy Potato")
    }
}

/// System that says "Buy Tomato"

pub struct BuyTomatoSystem;

impl<'a> System<'a> for BuyTomatoSystem {
    type SystemData = Write<'a, TomatoStore>;

    fn run(&mut self, _data: Self::SystemData) {
        println!("Buy Tomato")
    }
}

/// Batch controller that customizes how inner systems are executed

pub struct CustomBatchControllerSystem<'a, 'b> {
    accessor: BatchAccessor,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> BatchController<'a, 'b> for CustomBatchControllerSystem<'a, 'b> {
    // Leaving `BatchBuilderData` to `()` would make the dispatcher to panic since the run
    // function will fetch the `TomatoStore` like the `SayHelloSystem` does.
    // type BatchSystemData = ();
    type BatchSystemData = Read<'a, TomatoStore>;

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
        {
            // The scope is used to unload the resource before dispatching inner systems
            let _ts = data.0.fetch::<TomatoStore>();
        }
        println!("Batch execution");
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
