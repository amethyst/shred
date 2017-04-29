//! **Sh**ared **re**source **d**ispatcher
//!

#![allow(dead_code)]
#![allow(unused_variables)]
#![deny(unused_must_use)]
#![warn(missing_docs)]

extern crate fnv;
#[macro_use]
extern crate mopa;
extern crate rayon;

mod bitset;
mod cell;
mod res;

pub use res::{Fetch, FetchId, FetchIdMut, FetchMut, Resource, ResourceId, Resources};

/// Specifies which kind of access
/// a [`Task`] needs to a [`Resource`]
/// in order to be executed or a logical
/// dependency on another `Task`s result.
///
/// [`Task`]: trait.Task.html
/// [`Resource`]: trait.Resource.html
pub enum Dependency {
    // TODO: split up in separate lists
    /// The `Task` operates read-only
    /// on the specified dependency.
    Read(ResourceId),
    /// The `Task` may read and write
    /// from/to the specified dependency
    /// because it has exclusive access.
    Write(ResourceId),
    /// The `Task` logically depends
    /// on the work done by another one.
    Task(usize),
}

/// A `Task`, executed with a
/// set of required [`Resource`]s.
///
/// [`Resource`]: trait.Resource.html
pub trait Task {
    /// The resource bundle required
    /// to execute this task.
    ///
    /// To create such a resource bundle,
    /// simple derive `TaskData` for it.
    type TaskData;

    /// Executes the task with the required task
    /// data.
    fn work(&mut self, bundle: Self::TaskData);
}

/// A struct implementing
/// task data indicates that it
/// bundles some resources which are
/// required for the execution.
pub trait TaskData<'a> {
    /// Creates a new resource bundle
    /// by fetching the required resources
    /// from the [`Resources`] struct.
    ///
    /// # Contract
    ///
    /// Only fetch the resources you
    /// returned from `reads` / `writes`!
    ///
    /// [`Resources`]: trait.Resources.html
    fn fetch(res: &'a Resources) -> Self;

    /// A list of [`ResourceId`]s the bundle
    /// needs read access to in order to
    /// build the target resource bundle.
    ///
    /// # Contract
    ///
    /// Exactly return the dependencies you're
    /// going to `fetch`! Doing otherwise *will*
    /// cause a data race.
    ///
    /// This method is only executed once,
    /// thus the returned value may never change
    /// (otherwise it has no effect).
    unsafe fn reads() -> Vec<ResourceId>;

    /// A list of [`ResourceId`]s the bundle
    /// needs write access to in order to
    /// build the target resource bundle.
    ///
    /// # Contract
    ///
    /// Exactly return the dependencies you're
    /// going to `fetch`! Doing otherwise *will*
    /// cause a data race.
    ///
    /// This method is only executed once,
    /// thus the returned value may never change
    /// (otherwise it has no effect).
    unsafe fn writes() -> Vec<ResourceId>;
}
