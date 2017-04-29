extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{Fetch, FetchMut, Resource, Resources, TaskData};

#[derive(Debug)]
struct ResA;

impl Resource for ResA {}

#[derive(Debug)]
struct ResB;

impl Resource for ResB {}

#[derive(TaskData)]
pub struct AutoBundle<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

fn main() {
    let mut res = Resources::new();
    res.add(ResA);
    res.add(ResB);


    let mut bundle = AutoBundle::fetch(&res);

    *bundle.b = ResB;

    println!("{:?}", *bundle.a);
    println!("{:?}", *bundle.b);
}
