use shred::{Read, ResourceId, SystemData, World, Write};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

#[cfg(feature = "shred-derive")]
#[derive(SystemData)]
struct AutoBundle<'a> {
    a: Read<'a, ResA>,
    b: Write<'a, ResB>,
}

// We can even nest system data
#[cfg(feature = "shred-derive")]
#[derive(SystemData)]
struct Nested<'a> {
    inner: AutoBundle<'a>,
}

fn main() {
    let mut res = World::empty();
    res.insert(ResA);
    res.insert(ResB);

    {
        let mut bundle = AutoBundle::fetch(&res);

        *bundle.b = ResB;

        println!("{:?}", *bundle.a);
        println!("{:?}", *bundle.b);
    }

    {
        let nested = Nested::fetch(&res);

        println!("a: {:?}", *nested.inner.a);
    }
}

// The following is required for the example to compile without the
// `shred-derive` feature.

#[cfg(not(feature = "shred-derive"))]
struct AutoBundle<'a> {
    a: Read<'a, ResA>,
    b: Write<'a, ResB>,
}

#[cfg(not(feature = "shred-derive"))]
impl<'a> SystemData<'a> for AutoBundle<'a> {
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

#[cfg(not(feature = "shred-derive"))]
struct Nested<'a> {
    inner: AutoBundle<'a>,
}

#[cfg(not(feature = "shred-derive"))]
impl<'a> SystemData<'a> for Nested<'a> {
    fn setup(world: &mut World) {
        AutoBundle::<'_>::setup(world);
    }

    fn fetch(world: &'a World) -> Self {
        Self {
            inner: AutoBundle::<'_>::fetch(world),
        }
    }

    fn reads() -> Vec<ResourceId> {
        AutoBundle::<'_>::reads()
    }

    fn writes() -> Vec<ResourceId> {
        AutoBundle::<'_>::writes()
    }
}
