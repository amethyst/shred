use shred::{DispatcherBuilder, Read, ResourceId, System, SystemData, World, Write};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

#[cfg(feature = "shred-derive")]
#[derive(SystemData)]
struct Data<'a> {
    a: Read<'a, ResA>,
    b: Write<'a, ResB>,
}

struct EmptySystem;

impl<'a> System<'a> for EmptySystem {
    type SystemData = Data<'a>;

    fn run(&mut self, bundle: Data<'a>) {
        println!("{:?}", &*bundle.a);
        println!("{:?}", &*bundle.b);
    }
}

fn main() {
    let mut resources = World::empty();
    let mut dispatcher = DispatcherBuilder::new()
        .with(EmptySystem, "empty", &[])
        .build();
    dispatcher.setup(&mut resources);

    dispatcher.dispatch_seq(&resources);
}

// The following is required for the example to compile without the
// `shred-derive` feature.

#[cfg(not(feature = "shred-derive"))]
struct Data<'a> {
    a: Read<'a, ResA>,
    b: Write<'a, ResB>,
}

#[cfg(not(feature = "shred-derive"))]
impl<'a> SystemData<'a> for Data<'a> {
    fn setup(world: &mut World) {
        Read::<'_, ResA>::setup(world);
        Write::<'_, ResB>::setup(world);
    }

    fn fetch(world: &'a World) -> Self {
        Self {
            a: Read::<'_, ResA>::fetch(world),
            b: Write::<'_, ResB>::fetch(world),
        }
    }

    fn reads() -> Vec<ResourceId> {
        Read::<'_, ResA>::reads()
    }

    fn writes() -> Vec<ResourceId> {
        Write::<'_, ResB>::writes()
    }
}
