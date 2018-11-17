extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{DispatcherBuilder, Read, Resources, System, Write};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

#[derive(SystemData)]
struct Data {
    a: Read<ResA>,
    b: Write<ResB>,
}

struct EmptySystem(*mut i8); // System is not thread-safe

impl System for EmptySystem {
    type SystemData = Data;

    fn run(&mut self, bundle: Data) {
        println!("thread local: {:?}", &*bundle.a);
        println!("thread local: {:?}", &*bundle.b);
    }
}

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
    let mut x = 5;

    let mut resources = Resources::new();
    resources.insert(ResA);
    resources.insert(ResB);
    let mut dispatcher = DispatcherBuilder::new()
        .with(PrintSystem, "print", &[]) // Adds a system "print" without dependencies
        .with_thread_local(EmptySystem(&mut x))
        .build_async(resources);

    dispatcher.dispatch();
    dispatcher.wait();

    dispatcher.dispatch();
    dispatcher.wait();
}
