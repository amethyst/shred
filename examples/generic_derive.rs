#![allow(unused)]

extern crate shred;
#[macro_use]
extern crate shred_derive;

use std::fmt::Debug;

use shred::{Read, Resource, Write};

trait Hrtb<'a> {}

#[derive(SystemData)]
struct VeryCustomDerive<T: Debug + Resource + for<'b> Hrtb<'b>> {
    _b: Write<T>,
}

#[derive(SystemData)]
struct SomeTuple<T: Debug + Resource>(Read<T>);

#[derive(SystemData)]
struct WithWhereClause<T>
where
    T: Resource,
{
    k: Read<T>,
}

fn main() {}
