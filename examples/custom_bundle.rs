extern crate shred;

use shred::{Fetch, FetchMut, ResourceId, Resources, SystemData};

#[derive(Debug)]
struct ResA;

#[derive(Debug)]
struct ResB;

struct ExampleBundle<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

impl<'a> SystemData<'a> for ExampleBundle<'a> {
    fn fetch(res: &'a Resources) -> Self {
        ExampleBundle {
            a: res.fetch(()),
            b: res.fetch_mut(()),
        }
    }

    unsafe fn reads() -> Vec<ResourceId> {
        vec![ResourceId::new::<ResA>()]
    }

    unsafe fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<ResB>()]
    }
}

fn main() {
    let mut res = Resources::new();
    res.add(ResA, ());
    res.add(ResB, ());


    let mut bundle = ExampleBundle::fetch(&res);
    *bundle.b = ResB;

    println!("{:?}", *bundle.a);
    println!("{:?}", *bundle.b);

}
