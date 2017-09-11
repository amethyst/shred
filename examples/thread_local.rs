extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{DispatcherBuilder, Fetch, FetchMut, Resources, System, ThreadLocal};

#[derive(Debug)]
struct ResA;

#[derive(Debug)]
struct ResB;

struct ResThreadLocal { // Resource is not thread-safe
    non_send_sync_stuff: *mut u8,
}

#[derive(SystemData)]
struct Data<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
    thread_local: Fetch<'a, ThreadLocal<ResThreadLocal>>,
}

struct EmptySystem(*mut i8); // System is not thread-safe

impl<'a> System<'a> for EmptySystem {
    type SystemData = Data<'a>;

    fn run(&mut self, bundle: Data<'a>) {
        println!("{:?}", &*bundle.a);
        println!("{:?}", &*bundle.b);

        if bundle
            .thread_local
            .get()
            .map(|x| x.non_send_sync_stuff == 0x20 as *mut u8)
            .unwrap_or(false)
        {
            println!("Thread local resource points to 0x20");
        }
    }
}

fn main() {
    let mut x = 5;

    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add_thread_local(EmptySystem(&mut x))
        .build();
    resources.add(ResA);
    resources.add(ResB);
    resources.add(ThreadLocal::new(ResThreadLocal { non_send_sync_stuff: 0x20 as *mut u8 }));

    dispatcher.dispatch(&mut resources);
}
