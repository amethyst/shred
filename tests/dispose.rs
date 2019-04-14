extern crate shred;

use shred::*;

struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = Write<'a, i32>;

    fn run(&mut self, mut data: Self::SystemData) {
        *data += 2;
    }

    fn setup(&mut self, res: &mut Resources) {
        let _ = res.entry::<i32>().or_insert(5);
    }

    fn dispose(self, res: &mut Resources) {
        *res.fetch_mut::<i32>() = 0;
    }
}

#[test]
fn test_dispose() {
    let mut res = Resources::new();

    let mut dispatcher = DispatcherBuilder::new().with(Sys, "sys", &[]).build();

    dispatcher.setup(&mut res);

    assert_eq!(*res.fetch::<i32>(), 5);

    dispatcher.dispatch(&res);

    assert_eq!(*res.fetch::<i32>(), 7);

    dispatcher.dispatch(&res);

    assert_eq!(*res.fetch::<i32>(), 9);

    dispatcher.dispose(&mut res);

    assert_eq!(*res.fetch::<i32>(), 0);
}
