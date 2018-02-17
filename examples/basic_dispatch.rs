extern crate shred;

use shred::{DispatcherBuilder, Fetch, FetchMut, Resources, System};

#[derive(Debug, Default)]
struct ResA;

// A resource usually has a `Default` implementation
// which will be used if the resource has not been added.
#[derive(Debug, Default)]
struct ResB;

struct PrintSystem;

impl<'a> System<'a> for PrintSystem {
    type SystemData = (Fetch<'a, ResA>, FetchMut<'a, ResB>);

    fn run(&mut self, data: Self::SystemData) {
        let (a, mut b) = data;

        println!("{:?}", &*a);
        println!("{:?}", &*b);

        *b = ResB; // We can mutate ResB here
                   // because it's `FetchMut`.
    }
}

fn main() {
    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .with(PrintSystem, "print", &[]) // Adds a system "print" without dependencies
        .build();
    resources.add(ResA);
    resources.add(ResB);

    dispatcher.dispatch(&resources);

    resources.maintain();
}
