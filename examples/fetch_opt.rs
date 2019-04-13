extern crate shred;

use shred::{DispatcherBuilder, Read, ReadExpect, System, World, Write};

#[derive(Debug, Default)]
struct ResA;

// `ResB` does not implement `Default`.
#[derive(Debug)]
struct ResB;

struct ResWithoutSensibleDefault {
    magic_number_that_we_cant_compute: u32,
}

struct PrintSystem;

impl<'a> System<'a> for PrintSystem {
    // We can simply use `Option<Read>` or `Option<Write>` if a resource
    // isn't strictly required or can't be created (by a `Default` implementation).
    type SystemData = (
        Read<'a, ResA>,
        Option<Write<'a, ResB>>,
        // WARNING: using `ReadExpect` might lead to a panic!
        // If `ResWithoutSensibleDefault` does not exist, fetching will `panic!`.
        ReadExpect<'a, ResWithoutSensibleDefault>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (a, mut b, expected) = data;

        println!("{:?}", &*a);

        if let Some(ref mut x) = b {
            println!("{:?}", &**x);

            **x = ResB;
        }

        println!(
            "Yeah, we have our magic number: {}",
            expected.magic_number_that_we_cant_compute
        );
    }
}

fn main() {
    let mut resources = World::empty();
    let mut dispatcher = DispatcherBuilder::new()
        .with(PrintSystem, "print", &[]) // Adds a system "print" without dependencies
        .build();

    // Will automatically insert `ResB` (the only one that has a default provider).
    dispatcher.setup(&mut resources);
    resources.insert(ResWithoutSensibleDefault {
        magic_number_that_we_cant_compute: 42,
    });

    // `ResB` is not in resources, but `PrintSystem` still works.
    dispatcher.dispatch(&resources);

    resources.insert(ResB);

    // Now `ResB` can be printed, too.
    dispatcher.dispatch(&resources);
}
