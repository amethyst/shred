extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{DispatcherBuilder, Fetch, FetchMut, Resource, Resources, System};

#[derive(Debug)]
struct ResA;

impl Resource for ResA {}

#[derive(Debug)]
struct ResB;

impl Resource for ResB {}

#[derive(SystemData)]
struct Data<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

struct EmptySystem;

impl<'a, C> System<'a, C> for EmptySystem {
    type SystemData = Data<'a>;

    fn work(&mut self, bundle: Data<'a>, _: C) {
        println!("{:?}", &*bundle.a);
        println!("{:?}", &*bundle.b);
    }
}

fn main() {
    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add(EmptySystem, "empty", &[])
        .finish();
    resources.add(ResA, ());
    resources.add(ResB, ());

    dispatcher.dispatch_seq(&mut resources, ());
}
