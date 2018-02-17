extern crate shred;

use shred::{Fetch, FetchMut, ResourceId, Resources, SystemData};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

struct ExampleBundle<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

impl<'a> SystemData<'a> for ExampleBundle<'a> {
    fn fetch(res: &'a Resources) -> Self {
        ExampleBundle {
            a: SystemData::fetch(res),
            b: SystemData::fetch(res),
        }
    }

    fn reads() -> Vec<ResourceId> {
        vec![ResourceId::new::<ResA>()]
    }

    fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<ResB>()]
    }
}

fn main() {
    let mut res = Resources::new();
    res.add(ResA);
    res.add(ResB);

    let mut bundle = ExampleBundle::fetch(&res);
    *bundle.b = ResB;

    println!("{:?}", *bundle.a);
    println!("{:?}", *bundle.b);
}
