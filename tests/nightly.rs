#![cfg(feature = "nightly")]

extern crate shred;

use shred::{FetchExpect, Resources, SystemData};

struct MyRes;

#[test]
#[should_panic(expected = "Tried to fetch a resource of type \"MyRes\"")]
fn try_helpful_panic() {
    let res = Resources::new();

    let _expect: FetchExpect<MyRes> = SystemData::fetch(&res);
}
