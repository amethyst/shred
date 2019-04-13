extern crate shred;

use shred::{Read, ResourceId, SystemData, World, Write};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

struct ExampleBundle<'a> {
    a: Read<'a, ResA>,
    b: Write<'a, ResB>,
}

impl<'a> SystemData<'a> for ExampleBundle<'a> {
    fn setup(res: &mut World) {
        res.entry().or_insert(ResA);
        res.entry().or_insert(ResB);
    }

    fn fetch(res: &'a World) -> Self {
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
    let mut res = World::empty();
    res.insert(ResA);
    res.insert(ResB);

    let mut bundle = ExampleBundle::fetch(&res);
    *bundle.b = ResB;

    println!("{:?}", *bundle.a);
    println!("{:?}", *bundle.b);
}
