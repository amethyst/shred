#![allow(unused)]

extern crate shred;
#[macro_use]
extern crate shred_derive;

use std::fmt::Debug;

use shred::{Read, Resource, Write};

trait Hrtb<'a> {}

#[derive(StaticSystemData)]
struct VeryCustomDerive<'a, T: Debug + Resource + for<'b> Hrtb<'b>> {
    _b: Write<'a, T>,
}

#[derive(StaticSystemData)]
struct SomeTuple<'a, T: Debug + Resource>(Read<'a, T>);

#[derive(StaticSystemData)]
struct WithWhereClause<'a, T>
where
    T: Resource,
{
    k: Read<'a, T>,
}

fn main() {}
