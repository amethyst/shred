extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{DispatcherBuilder, Fetch, FetchMut, Resources, System};

#[derive(Debug)]
struct ResA;

#[derive(Debug)]
struct ResB;

#[derive(SystemData)]
struct Data<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

struct EmptySystem(*mut i8); // System is not thread-safe

impl<'a> System<'a> for EmptySystem {
    type SystemData = Data<'a>;

    fn run(&mut self, bundle: Data<'a>) {
        println!("thread local: {:?}", &*bundle.a);
        println!("thread local: {:?}", &*bundle.b);
    }
}

struct PrintSystem;

impl<'a> System<'a> for PrintSystem {
    type SystemData = (Fetch<'a, ResA>, FetchMut<'a, ResB>);

    fn run(&mut self, data: Self::SystemData) {
        let (a, mut b) = data;

        println!("{:?}", &*a);
        println!("{:?}", &*b);

        *b = ResB; // We can mutate ResB here
        // because it's `FetchMut`.
    }
}

fn main() {
    let mut x = 5;

    let mut resources = Resources::new();
    resources.add(ResA);
    resources.add(ResB);
    let mut dispatcher = DispatcherBuilder::new()
        .with(PrintSystem, "print", &[]) // Adds a system "print" without dependencies
        .with_thread_local(EmptySystem(&mut x))
        .build_async(resources);

    dispatcher.dispatch();
    dispatcher.wait();

    dispatcher.dispatch();
    dispatcher.wait();
}
