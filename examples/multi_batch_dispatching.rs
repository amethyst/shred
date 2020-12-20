//! This example shows how to use and define a multi-batch dispatcher.
//!
//! It allows to influence how many times a set of systems gets dispatched.
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
//! This is done by defining `Run3Times` which decides that the inner systems should be run 3
//! times. This is similar to the `batch_dispatching.rs` example, but that one uses a more flexible
//! (but also more verbose) way of doing it.

use shred::{
    DispatcherBuilder, Read, System, World, Write, MultiDispatchController, MultiDispatcher,
};
use std::{thread::sleep, time::Duration};

fn main() {
    let mut dispatcher = DispatcherBuilder::new()
        .with(SayHelloSystem, "say_hello_system", &[])
        .with_batch(
            MultiDispatcher::new(Run3Times),
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

#[derive(Default)]
struct Run3Times;

impl<'a> MultiDispatchController<'a> for Run3Times {
    type SystemData = Read<'a, TomatoStore>;

    fn plan(&mut self, _data: Self::SystemData) -> usize {
        3
    }
}
