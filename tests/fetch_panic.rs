extern crate shred;

use shred::{ReadExpect, SystemData, World};

struct MyRes;

#[test]
#[should_panic(
    expected = r#"Tried to fetch resource of type `MyRes`[^1] from the `World`, but the resource does not exist.

You may ensure the resource exists through one of the following methods:

* Inserting it when the world is created: `world.insert(..)`.
* If the resource implements `Default`, include it in a system's `SystemData`, and ensure the system is registered in the dispatcher.
* If the resource does not implement `Default`, insert it in the world during `System::setup`.

[^1]: Full type name: `fetch_panic::MyRes`"#
)]
fn try_helpful_panic() {
    let res = World::empty();

    let _expect: ReadExpect<MyRes> = SystemData::fetch(&res);
}
