extern crate shred;
#[macro_use]
extern crate shred_derive;

use std::fmt::Debug;

use shred::{FetchMut, Resource};

#[derive(SystemData)]
struct VeryCustomDerive<'a, T: Debug + Resource> {
    _b: FetchMut<'a, T>,
}

fn main() {}
