extern crate shred;

use shred::{Fetch, FetchMut, ResourceId, Prefetch, Resources, SystemData};

#[derive(Debug)]
struct ResA;

#[derive(Debug)]
struct ResB;

struct ExampleBundle<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

impl<'a> SystemData<'a> for ExampleBundle<'a> {
    type Prefetch = ();

    fn fetch(res: &'a Resources, id: usize) -> Self {
        ExampleBundle {
            a: res.fetch(id),
            b: res.fetch_mut(id),
        }
    }

    fn reads(id: usize) -> Vec<ResourceId> {
        vec![ResourceId::new_with_id::<ResA>(id)]
    }

    fn writes(id: usize) -> Vec<ResourceId> {
        vec![ResourceId::new_with_id::<ResB>(id)]
    }
}

struct ExamplePrefetch<'a> {
    a: <Fetch<'a, ResA> as SystemData<'a>>::Prefetch,
    b: <FetchMut<'a, ResB> as SystemData<'a>>::Prefetch,
}

impl<'a> Prefetch<'a> for ExamplePrefetch<'a> {
    type Data = ExampleBundle<'a>;

    fn prefetch(res: &'a Resources) -> Self {
        ExamplePrefetch {
            a: Prefetch::prefetch(res),
            b: Prefetch::prefetch(res),
        }
    }

    fn fetch(&'a mut self) -> Self::Data {
        ExampleBundle {
            a: self.a.fetch(),
            b: self.b.fetch(),
        }
    }
}

fn main() {
    let mut res = Resources::new();
    res.add(ResA);
    res.add(ResB);


    let mut bundle = ExampleBundle::fetch(&res, 0);
    *bundle.b = ResB;

    println!("{:?}", *bundle.a);
    println!("{:?}", *bundle.b);
}
