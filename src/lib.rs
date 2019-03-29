//! **Sh**ared **re**source **d**ispatcher
//!
//! This library allows to dispatch
//! systems, which can have interdependencies,
//! shared and exclusive resource access, in parallel.
//!
//! # Examples
//!
//! ```rust
//! extern crate shred;
//! #[macro_use]
//! extern crate shred_derive;
//!
//! use shred::{DispatcherBuilder, Read, Resource, ResourceId, System, SystemData, World, Write};
//!
//! #[derive(Debug, Default)]
//! struct ResA;
//!
//! #[derive(Debug, Default)]
//! struct ResB;
//!
//! #[derive(SystemData)]
//! struct Data<'a> {
//!     a: Read<'a, ResA>,
//!     b: Write<'a, ResB>,
//! }
//!
//! struct EmptySystem;
//!
//! impl<'a> System<'a> for EmptySystem {
//!     type SystemData = Data<'a>;
//!
//!     fn run(&mut self, bundle: Data<'a>) {
//!         println!("{:?}", &*bundle.a);
//!         println!("{:?}", &*bundle.b);
//!     }
//! }
//!
//! fn main() {
//!     let mut resources = World::new();
//!     let mut dispatcher = DispatcherBuilder::new()
//!         .with(EmptySystem, "empty", &[])
//!         .build();
//!     resources.insert(ResA);
//!     resources.insert(ResB);
//!
//!     dispatcher.dispatch(&mut resources);
//! }
//! ```
//!
//! Once you are more familiar with how system data and parallelization works,
//! you can take look at a more flexible and performant way to dispatch:
//! `ParSeq`. Using it is bit trickier, but it allows dispatching without any
//! virtual function calls.

#![cfg_attr(feature = "nightly", feature(core_intrinsics))]
#![deny(unused_must_use)]
#![warn(missing_docs)]

extern crate arrayvec;
extern crate hashbrown;
#[macro_use]
extern crate mopa;
#[cfg(feature = "parallel")]
extern crate rayon;
extern crate smallvec;

pub mod cell;

mod dispatch;
mod meta;
mod system;
mod world;

#[cfg(feature = "parallel")]
pub use crate::dispatch::AsyncDispatcher;
#[cfg(feature = "parallel")]
pub use crate::dispatch::{Par, ParSeq, RunWithPool, Seq};
pub use crate::{
    dispatch::{Dispatcher, DispatcherBuilder},
    meta::{CastFrom, MetaIter, MetaIterMut, MetaTable},
    system::{
        Accessor, AccessorCow, DynamicSystemData, RunNow, RunningTime, StaticAccessor, System,
        SystemData,
    },
    world::{
        DefaultProvider, Entry, Fetch, FetchMut, PanicHandler, Read, ReadExpect, Resource,
        ResourceId, SetupHandler, World, Write, WriteExpect,
    },
};
