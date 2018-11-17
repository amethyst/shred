extern crate shred;

use shred::{DispatcherBuilder, Read, Resources, System, Write};

#[derive(Debug, Default)]
struct ResA;

// A resource usually has a `Default` implementation
// which will be used if the resource has not been added.
#[derive(Debug, Default)]
struct ResB;

struct PrintSystem;

impl System for PrintSystem {
    type SystemData = (Read<ResA>, Write<ResB>);

    fn run(&mut self, data: Self::SystemData) {
        let (a, mut b) = data;

        println!("{:?}", &*a);
        println!("{:?}", &*b);

        *b = ResB; // We can mutate ResB here
                   // because it's `Write`.
    }
}

fn main() {
    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .with(PrintSystem, "print", &[]) // Adds a system "print" without dependencies
        .build();
    dispatcher.setup(&mut resources);

    // Dispatch as often as you want to
    dispatcher.dispatch(&resources);
    dispatcher.dispatch(&resources);
    // ...
}
