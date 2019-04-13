use shred::*;

struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = Write<'a, i32>;

    fn run(&mut self, mut data: Self::SystemData) {
        *data += 2;
    }

    fn setup(&mut self, world: &mut World) {
        let _ = world.entry::<i32>().or_insert(5);
    }

    fn dispose(self, world: &mut World) {
        *world.fetch_mut::<i32>() = 0;
    }
}

#[test]
fn test_dispose() {
    let mut world = World::empty();

    let mut dispatcher = DispatcherBuilder::new().with(Sys, "sys", &[]).build();

    dispatcher.setup(&mut world);

    assert_eq!(*world.fetch::<i32>(), 5);

    dispatcher.dispatch(&world);

    assert_eq!(*world.fetch::<i32>(), 7);

    dispatcher.dispatch(&world);

    assert_eq!(*world.fetch::<i32>(), 9);

    dispatcher.dispose(&mut world);

    assert_eq!(*world.fetch::<i32>(), 0);
}
