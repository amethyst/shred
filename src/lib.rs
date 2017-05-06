//! **Sh**ared **re**source **d**ispatcher
//!

#![deny(unused_must_use)]
#![warn(missing_docs)]

extern crate fnv;
#[macro_use]
extern crate mopa;
extern crate rayon;

mod bitset;
mod cell;
mod dispatch;
mod res;
mod task;

pub use dispatch::{Dispatcher, DispatcherBuilder};
pub use res::{Fetch, FetchId, FetchIdMut, FetchMut, Resource, ResourceId, Resources};
pub use task::{Task, TaskData};
