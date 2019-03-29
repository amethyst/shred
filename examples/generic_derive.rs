#![allow(unused)]

extern crate shred;
#[macro_use]
extern crate shred_derive;

use std::fmt::Debug;

use shred::{Read, Resource, ResourceId, SystemData, World, Write};

trait Hrtb<'a> {}

#[derive(SystemData)]
struct VeryCustomDerive<'a, T: Debug + Resource + for<'b> Hrtb<'b>> {
    _b: Write<'a, T>,
}

#[derive(SystemData)]
struct SomeTuple<'a, T: Debug + Resource>(Read<'a, T>);

#[derive(SystemData)]
struct WithWhereClause<'a, T>
where
    T: Resource,
{
    k: Read<'a, T>,
}

fn main() {}
