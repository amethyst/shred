#![allow(unused)]

extern crate shred;
#[macro_use]
extern crate shred_derive;

use std::fmt::Debug;

use shred::{Fetch, FetchMut, Resource};

#[derive(SystemData)]
struct VeryCustomDerive<'a, T: Debug + Resource> {
    _b: FetchMut<'a, T>,
}

#[derive(SystemData)]
struct SomeTuple<'a, T: Debug + Resource>(Fetch<'a, T>);

fn main() {}
