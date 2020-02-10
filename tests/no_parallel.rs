#![cfg(not(feature = "parallel"))]

use std::rc::Rc;

use shred::World;

#[derive(Default, PartialEq)]
struct ResNonSend(Rc<u32>);

#[test]
fn non_send_resource_is_accepted() {
    let mut world = World::empty();
    world.insert(ResNonSend(Rc::new(123)));

    let res_non_send = world.fetch::<ResNonSend>();
    assert_eq!(123, *res_non_send.0);
}
