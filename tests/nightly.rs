#![cfg(feature = "nightly")]

extern crate shred;

use shred::{ReadExpect, SystemData, World};

struct MyRes;

#[test]
#[should_panic(expected = "Tried to fetch a resource of type \"MyRes\"")]
fn try_helpful_panic() {
    let res = World::new();

    let _expect: ReadExpect<MyRes> = SystemData::fetch(&res);
}
