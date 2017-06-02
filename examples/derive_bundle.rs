extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{Fetch, FetchMut, Resources, SystemData};

#[derive(Debug)]
struct ResA;

#[derive(Debug)]
struct ResB;

#[derive(SystemData)]
pub struct AutoBundle<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

// We can even nest system data
#[derive(SystemData)]
struct Nested<'a> {
    inner: AutoBundle<'a>,
}

fn main() {
    let mut res = Resources::new();
    res.add(ResA, 0);
    res.add(ResB, 0);


    {
        let mut bundle = AutoBundle::fetch(&res, 0);

        *bundle.b = ResB;

        println!("{:?}", *bundle.a);
        println!("{:?}", *bundle.b);
    }

    {
        let nested = Nested::fetch(&res, 0);

        println!("a: {:?}", *nested.inner.a);
    }
}
