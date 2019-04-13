extern crate shred;

use shred::{DispatcherBuilder, Read, System, World, Write};

#[derive(Debug, Default)]
struct ResA;

// A resource usually has a `Default` implementation
// which will be used if the resource has not been added.
#[derive(Debug, Default)]
struct ResB;

struct PrintSystem;

impl<'a> System<'a> for PrintSystem {
    type SystemData = (Read<'a, ResA>, Write<'a, ResB>);

    fn run(&mut self, data: Self::SystemData) {
        let (a, mut b) = data;

        println!("{:?}", &*a);
        println!("{:?}", &*b);

        *b = ResB; // We can mutate ResB here
                   // because it's `Write`.
    }
}

fn main() {
    let mut resources = World::empty();
    let mut dispatcher = DispatcherBuilder::new()
        .with(PrintSystem, "print", &[]) // Adds a system "print" without dependencies
        .build();
    dispatcher.setup(&mut resources);

    // Dispatch as often as you want to
    dispatcher.dispatch(&resources);
    dispatcher.dispatch(&resources);
    // ...
}
