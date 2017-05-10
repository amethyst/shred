extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{DispatcherBuilder, Fetch, FetchMut, Resource, Resources};

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

#[derive(Clone)]
struct EmptySystem;

fn work<'a>(_: &mut EmptySystem, bundle: Data<'a>) {
    println!("{:?}", &*bundle.a);
    println!("{:?}", &*bundle.b);
}


fn main() {
    let mut resources = Resources::new();
    resources.add(ResA, ());
    resources.add(ResB, ());
    
    let mut dispatcher = DispatcherBuilder::new()
        .add("empty", work, EmptySystem, &[])
        .finish();

    dispatcher.dispatch(&resources);
}
