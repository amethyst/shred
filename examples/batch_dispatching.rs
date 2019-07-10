//! This example show how to use the Batch dispatcher.
//!
//! The Batch is a feature that allow to control the dispatching of a group of
//! systems.
//!
//! Specifically here we have three Systems
//! - **SayHelloSystem**: Which is directly registered under the main
//!   dispatcher.
//! - **BuyTomatoSystem** and **BuyPotatoSystem** are registered in the batch.
//!
//! Notice that none of them is using any direct dependency.
//! The SayHelloSystem is requesting the resources **TomatoStore** and
//! **PotatoStore**, which are also requested by the other two systems inside
//! the batch and by the batch controller itself.
//!
//! This to demonstrate that the dispatcher is able to correctly organize, their
//! execution.
//!
//! Is also showed how to use a custom BatchControllerSystem.
//! Check the **CustomBatchControllerSystem** which execute its inner Systems
//! three times.


use shred::{
    AccessorCow, BatchAccessor, BatchController, BatchUncheckedWorld, Dispatcher,
    DispatcherBuilder, Read, RunningTime, System, World, Write,
};
use std::thread::sleep;
use std::time::Duration;

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

// Say Hello Sytem

pub struct SayHelloSystem;

impl<'a> System<'a> for SayHelloSystem {
    type SystemData = (Write<'a, PotatoStore>, Write<'a, TomatoStore>);

    fn run(&mut self, _data: Self::SystemData) {
        println!("Hello!")
    }
}

// BuyPotatoSystem

pub struct BuyPotatoSystem;

impl<'a> System<'a> for BuyPotatoSystem {
    type SystemData = (Write<'a, PotatoStore>);

    fn run(&mut self, _data: Self::SystemData) {
        println!("Buy Potato")
    }
}

// Buy Tomato System

pub struct BuyTomatoSystem;

impl<'a> System<'a> for BuyTomatoSystem {
    type SystemData = (Write<'a, PotatoStore>);

    fn run(&mut self, _data: Self::SystemData) {
        println!("Buy Tomato")
    }
}

// TEST custom Batch Controller

pub struct CustomBatchControllerSystem<'a, 'b> {
    accessor: BatchAccessor,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> BatchController<'a, 'b> for CustomBatchControllerSystem<'a, 'b> {
    // Using this `BatchBuilderData = ()` make the dispatcher to panic since the run
    // function will fetch the TomatoStore like the SayHelloSystem does.
    // type BatchSystemData = ();
    type BatchSystemData = (Read<'a, TomatoStore>);

    fn create(accessor: BatchAccessor, dispatcher: Dispatcher<'a, 'b>) -> Self {
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
            // Unload before dispatch the inner dispatcher
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
