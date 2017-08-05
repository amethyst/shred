extern crate shred;

use shred::{DispatcherBuilder, Fetch, FetchMut, Prefetch, Resources, System, SystemData};

#[derive(Debug)]
struct ResA;

#[derive(Debug)]
struct ResB;

struct PrintSystem<'a> {
    prefetch: <(Fetch<'a, ResA>, FetchMut<'a, ResB>) as SystemData<'a>>::Prefetch,
}

impl<'a> PrintSystem<'a> {
    fn new(res: &'a Resources) -> Self {
        PrintSystem {
            prefetch: Prefetch::prefetch(res),
        }
    }
}

impl<'a, 'b> System<'a> for PrintSystem<'b> {
    type SystemData = (Fetch<'a, ResA>, FetchMut<'a, ResB>);

    fn fetch_data(&mut self, res: &'a Resources) -> Self::SystemData {
        self.prefetch.fetch()
    }

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
        .add(PrintSystem::new(&resources), "print", &[]) // Adds a system "print" without dependencies
        .build();
    resources.add(ResA);
    resources.add(ResB);

    dispatcher.dispatch(&mut resources);
}
