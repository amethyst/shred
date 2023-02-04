//! Helper module for some internals, most users don't need to interact with it.
//!
//! Note: The implementation of `TrustCell` has been replaced with re-exporting [`atomic_refcell::AtomicRefCell`].

pub use atomic_refcell::{AtomicRef as Ref, AtomicRefCell as TrustCell, AtomicRefMut as RefMut};
