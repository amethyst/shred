extern crate shred;

use shred::{DispatcherBuilder, Read, ResourceId, System, SystemData, World, Write};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

#[derive(SystemData)]
struct Data<'a> {
    a: Read<'a, ResA>,
    b: Write<'a, ResB>,
}

struct EmptySystem(*mut i8); // System is not thread-safe

impl<'a> System<'a> for EmptySystem {
    type SystemData = Data<'a>;

    fn run(&mut self, bundle: Data<'a>) {
        println!("{:?}", &*bundle.a);
        println!("{:?}", &*bundle.b);
    }
}

fn main() {
    let mut x = 5;

    let mut resources = World::empty();
    let mut dispatcher = DispatcherBuilder::new()
        .with_thread_local(EmptySystem(&mut x))
        .build();
    dispatcher.setup(&mut resources);

    dispatcher.dispatch(&resources);
}
