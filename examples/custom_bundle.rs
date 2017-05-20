extern crate shred;

use std::any::TypeId;

use shred::{Fetch, FetchMut, Resource, ResourceId, Resources, SystemData};

#[derive(Debug)]
struct ResA;

impl Resource for ResA {}

#[derive(Debug)]
struct ResB;

impl Resource for ResB {}

struct ExampleBundle<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

impl<'a> SystemData<'a> for ExampleBundle<'a> {
    fn fetch(res: &'a Resources) -> Self {
        ExampleBundle {
            a: res.fetch(0),
            b: res.fetch_mut(0),
        }
    }

    unsafe fn reads() -> Vec<ResourceId> {
        vec![(TypeId::of::<ResA>(), 0)]
    }

    unsafe fn writes() -> Vec<ResourceId> {
        vec![(TypeId::of::<ResB>(), 0)]
    }
}

fn main() {
    let mut res = Resources::new();
    res.add(ResA, 0);
    res.add(ResB, 0);


    let mut bundle = ExampleBundle::fetch(&res);
    *bundle.b = ResB;

    println!("{:?}", *bundle.a);
    println!("{:?}", *bundle.b);

}
