extern crate shred;

use shred::{Read, ResourceId, Resources, SystemData, Write, SystemFetch};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

struct ExampleBundle {
    a: Read<ResA>,
    b: Write<ResB>,
}

impl SystemData for ExampleBundle {
    fn setup(res: &mut Resources) {
        res.entry().or_insert(ResA);
        res.entry().or_insert(ResB);
    }

    fn fetch<'r>(res: &'r Resources) -> SystemFetch<'r, Self> {
        SystemFetch::from(ExampleBundle {
            a: SystemData::fetch(res).into_inner(),
            b: SystemData::fetch(res).into_inner(),
        })
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
    res.insert(ResA);
    res.insert(ResB);

    let mut bundle = ExampleBundle::fetch(&res).into_inner();
    *bundle.b = ResB;

    println!("{:?}", *bundle.a);
    println!("{:?}", *bundle.b);
}
