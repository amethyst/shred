extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{DispatcherBuilder, Fetch, FetchMut, Resource, Resources, Task};

#[derive(Debug)]
struct ResA;

impl Resource for ResA {}

#[derive(Debug)]
struct ResB;

impl Resource for ResB {}

#[derive(TaskData)]
struct Data<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

struct EmptyTask;

impl<'a> Task<'a> for EmptyTask {
    type TaskData = Data<'a>;

    fn work(&mut self, bundle: Data<'a>) {
        println!("{:?}", &*bundle.a);
        println!("{:?}", &*bundle.b);
    }
}


fn main() {
    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add(EmptyTask, "empty", &[])
        .finish();
    resources.add(ResA, ());
    resources.add(ResB, ());

    dispatcher.dispatch(&mut resources);
}
