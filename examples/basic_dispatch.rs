extern crate shred;

use shred::{DispatcherBuilder, Fetch, FetchMut, Resource, Resources, System};

#[derive(Debug)]
struct ResA;

impl Resource for ResA {}

#[derive(Debug)]
struct ResB;

impl Resource for ResB {}

struct PrintSystem;

// Systems should be generic over the
// context if possible, so it's easy
// to introduce one.
impl<'a, C> System<'a, C> for PrintSystem {
    type SystemData = (Fetch<'a, ResA>, FetchMut<'a, ResB>);

    fn work(&mut self, data: Self::SystemData, _: C) {
        let (a, mut b) = data;

        println!("{:?}", &*a);
        println!("{:?}", &*b);

        *b = ResB; // We can mutate ResB here
        // because it's `FetchMut`.
    }
}

fn main() {
    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add(PrintSystem, "print", &[]) // Adds a system "print" without dependencies
        .build();
    resources.add(ResA, ());
    resources.add(ResB, ());

    // We can even pass a context,
    // but we don't need one here
    // so we pass `()`.
    dispatcher.dispatch(&mut resources, ());
}
