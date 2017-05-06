//! **Sh**ared **re**source **d**ispatcher
//!
//! This library allows to dispatch
//! tasks, which can have interdependencies,
//! shared and exclusive resource access, in parallel.
//!
//! # Examples
//!
//! ```rust
//! extern crate shred;
//! #[macro_use]
//! extern crate shred_derive;
//!
//! use shred::{DispatcherBuilder, Fetch, FetchMut, Resource, Resources, Task};
//!
//! #[derive(Debug)]
//! struct ResA;
//!
//! impl Resource for ResA {}
//!
//! #[derive(Debug)]
//! struct ResB;
//!
//! impl Resource for ResB {}
//!
//! #[derive(TaskData)]
//! struct Data<'a> {
//!     a: Fetch<'a, ResA>,
//!     b: FetchMut<'a, ResB>,
//! }
//!
//! struct EmptyTask;
//!
//! impl<'a> Task<'a> for EmptyTask {
//!     type TaskData = Data<'a>;
//!
//!     fn work(&mut self, bundle: Data<'a>) {
//!         println!("{:?}", &*bundle.a);
//!         println!("{:?}", &*bundle.b);
//!     }
//! }
//!
//!
//! fn main() {
//!     let mut resources = Resources::new();
//!     let mut dispatcher = DispatcherBuilder::new()
//!         .add(EmptyTask, "empty", &[])
//!         .finish();
//!     resources.add(ResA, ());
//!     resources.add(ResB, ());
//!
//!     dispatcher.dispatch(&mut resources);
//! }
//! ```
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
