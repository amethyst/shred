extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{Read, Resources, SystemData, Write};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

#[derive(SystemData)]
struct AutoBundle {
    a: Read<ResA>,
    b: Write<ResB>,
}

// We can even nest system data
#[derive(SystemData)]
struct Nested {
    inner: AutoBundle,
}

fn main() {
    let mut res = Resources::new();
    res.insert(ResA);
    res.insert(ResB);

    {
        let mut bundle = AutoBundle::fetch(&res).into_inner();

        *bundle.b = ResB;

        println!("{:?}", *bundle.a);
        println!("{:?}", *bundle.b);
    }

    {
        let nested = Nested::fetch(&res).into_inner();

        println!("a: {:?}", *nested.inner.a);
    }
}
