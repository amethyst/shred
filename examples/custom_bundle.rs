extern crate shred;

use shred::{Read, ResourceId, Resources, StaticSystemData, Write};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

struct ExampleBundle<'a> {
    a: Read<'a, ResA>,
    b: Write<'a, ResB>,
}

impl<'a> StaticSystemData<'a> for ExampleBundle<'a> {
    fn setup(res: &mut Resources) {
        res.entry().or_insert(ResA);
        res.entry().or_insert(ResB);
    }

    fn fetch(res: &'a Resources) -> Self {
        ExampleBundle {
            a: StaticSystemData::fetch(res),
            b: StaticSystemData::fetch(res),
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
    res.insert(ResA);
    res.insert(ResB);

    let mut bundle = ExampleBundle::fetch(&res);
    *bundle.b = ResB;

    println!("{:?}", *bundle.a);
    println!("{:?}", *bundle.b);
}
