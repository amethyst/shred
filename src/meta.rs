use std::{any::TypeId, collections::hash_map::Entry, marker::PhantomData};

use ahash::AHashMap as HashMap;

use crate::cell::{AtomicRef, AtomicRefMut};
use crate::{Resource, ResourceId, World};

/// This implements `Send` and `Sync` unconditionally.
/// (the trait itself doesn't need to have these bounds and the
/// resources are already guaranteed to fulfill it).
struct Invariant<T: ?Sized>(*mut T);

unsafe impl<T> Send for Invariant<T> where T: ?Sized {}

unsafe impl<T> Sync for Invariant<T> where T: ?Sized {}

/// Helper trait for the `MetaTable`.
///
/// This trait is required to be implemented for a trait to be compatible with
/// the meta table.
///
/// # Safety
///
/// The produced pointer must have the same provenance and address as the
/// provided pointer and a vtable that is valid for the type `T`.
///
/// # Examples
///
/// ```
/// use shred::CastFrom;
///
/// trait Foo {
///     fn foo1(&self);
///     fn foo2(&mut self, x: i32) -> i32;
/// }
///
/// unsafe impl<T> CastFrom<T> for dyn Foo
/// where
///     T: Foo + 'static,
/// {
///     fn cast(t: *mut T) -> *mut (dyn Foo + 'static) {
///         t
///     }
/// }
/// ```
pub unsafe trait CastFrom<T> {
    /// Casts a concrete pointer to `T` to a trait object pointer.
    fn cast(t: *mut T) -> *mut Self;
}

/// An iterator for the `MetaTable`.
pub struct MetaIter<'a, T: ?Sized + 'a> {
    vtable_fns: &'a [fn(*mut ()) -> *mut T],
    index: usize,
    tys: &'a [TypeId],
    // `MetaIter` is invariant over `T`
    marker: PhantomData<Invariant<T>>,
    world: &'a World,
}

impl<'a, T> Iterator for MetaIter<'a, T>
where
    T: ?Sized + 'a,
{
    type Item = AtomicRef<'a, T>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        loop {
            let resource_id = match self.tys.get(self.index) {
                Some(&x) => ResourceId::from_type_id(x),
                None => return None,
            };

            let index = self.index;
            self.index += 1;

            // SAFETY: We just read the value and don't replace it.
            if let Some(res) = unsafe { self.world.try_fetch_internal(resource_id) } {
                let vtable_fn = self.vtable_fns[index];
                let trait_object = AtomicRef::map(res.borrow(), |res: &Box<dyn Resource>| {
                    let ptr: *const dyn Resource = Box::as_ref(res);
                    let trait_ptr = (vtable_fn)(ptr.cast::<()>().cast_mut());
                    // SAFETY: For a particular index we store a corresponding
                    // TypeId and vtable_fn in tys and vtable_fns respectively.
                    // We rely on `try_fetch_interal` returning a trait object
                    // with a concrete type that has the provided TypeId. The
                    // signature of the closure parameter of `AtomicRef::map`
                    // should ensure we aren't accidentally extending the
                    // lifetime here. Also see safety note in `MetaTable::get`.
                    unsafe { &*trait_ptr }
                });

                return Some(trait_object);
            }
        }
    }
}

/// A mutable iterator for the `MetaTable`.
pub struct MetaIterMut<'a, T: ?Sized + 'a> {
    vtable_fns: &'a [fn(*mut ()) -> *mut T],
    index: usize,
    tys: &'a [TypeId],
    // `MetaIterMut` is invariant over `T`
    marker: PhantomData<Invariant<T>>,
    world: &'a World,
}

impl<'a, T> Iterator for MetaIterMut<'a, T>
where
    T: ?Sized + 'a,
{
    type Item = AtomicRefMut<'a, T>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        loop {
            let resource_id = match self.tys.get(self.index) {
                Some(&x) => ResourceId::from_type_id(x),
                None => return None,
            };

            let index = self.index;
            self.index += 1;

            // Note: this relies on implementation details of
            // try_fetch_internal!
            // SAFETY: We don't swap out the Box or expose a mutable reference to it.
            if let Some(res) = unsafe { self.world.try_fetch_internal(resource_id) } {
                let vtable_fn = self.vtable_fns[index];
                let trait_object =
                    AtomicRefMut::map(res.borrow_mut(), |res: &mut Box<dyn Resource>| {
                        let ptr: *mut dyn Resource = Box::as_mut(res);
                        let trait_ptr = (vtable_fn)(ptr.cast::<()>());
                        // SAFETY: For a particular index we store a corresponding
                        // TypeId and vtable_fn in tys and vtable_fns respectively.
                        // We rely on `try_fetch_interal` returning a trait object
                        // with a concrete type that has the provided TypeId. The
                        // signature of the closure parameter of `AtomicRefMut::map`
                        // should ensure we aren't accidentally extending the
                        // lifetime here. Also see safety note in
                        // `MetaTable::get_mut`.
                        unsafe { &mut *trait_ptr }
                    });

                return Some(trait_object);
            }
        }
    }
}

/// Given an address and provenance, produces a pointer to a trait object for
/// which `CastFrom<T>` is implemented.
///
/// Returned pointer has:
/// * the provenance of the provided pointer
/// * the address of the provided pointer
/// * a vtable that is valid for the concrete type `T`
///
/// We exclusively operate on pointers here so we only need a single function
/// pointer in the meta-table for both `&T` and `&mut T` cases.
fn attach_vtable<TraitObject: ?Sized, T>(value: *mut ()) -> *mut TraitObject
where
    TraitObject: CastFrom<T> + 'static,
    T: core::any::Any,
{
    // NOTE: This should be equivalent to `Any::downcast_ref_unchecked` except
    // with pointers and we don't require `Any` trait but still require that the
    // types are 'static.
    let trait_ptr = <TraitObject as CastFrom<T>>::cast(value.cast::<T>());
    // TODO: use `.addr()` when stabilized
    // assert that address not changed (to catch some mistakes in CastFrom impl)
    assert!(
        core::ptr::eq(value, trait_ptr.cast::<()>()),
        "Bug: `CastFrom` did not cast `self`"
    );
    trait_ptr
}

/// The `MetaTable` which allows to store object-safe trait implementations for
/// resources.
///
/// For example, you have a trait `Foo` that is implemented by several
/// resources. You can register all the implementors using
/// `MetaTable::register`. Later on, you can iterate over all resources that
/// implement `Foo` without knowing their specific type.
///
/// # Examples
///
/// ```
/// use shred::{CastFrom, MetaTable, World};
///
/// trait Object {
///     fn method1(&self) -> i32;
///
///     fn method2(&mut self, x: i32);
/// }
///
/// unsafe impl<T> CastFrom<T> for dyn Object
/// where
///     T: Object + 'static,
/// {
///     fn cast(t: *mut T) -> *mut Self {
///         t
///     }
/// }
///
/// struct ImplementorA(i32);
///
/// impl Object for ImplementorA {
///     fn method1(&self) -> i32 {
///         self.0
///     }
///
///     fn method2(&mut self, x: i32) {
///         self.0 += x;
///     }
/// }
///
/// struct ImplementorB(i32);
///
/// impl Object for ImplementorB {
///     fn method1(&self) -> i32 {
///         self.0
///     }
///
///     fn method2(&mut self, x: i32) {
///         self.0 *= x;
///     }
/// }
///
/// let mut world = World::empty();
///
/// world.insert(ImplementorA(3));
/// world.insert(ImplementorB(1));
///
/// let mut table = MetaTable::<dyn Object>::new();
/// table.register::<ImplementorA>();
/// table.register::<ImplementorB>();
///
/// {
///     let mut iter = table.iter(&mut world);
///     assert_eq!(iter.next().unwrap().method1(), 3);
///     assert_eq!(iter.next().unwrap().method1(), 1);
/// }
/// ```
pub struct MetaTable<T: ?Sized> {
    // TODO: When `ptr_metadata` is stabilized we can use that to implement this
    // without a function call (and without trying to make assumptions about the
    // layout of trait object pointers). https://github.com/rust-lang/rust/issues/81513
    vtable_fns: Vec<fn(*mut ()) -> *mut T>,
    indices: HashMap<TypeId, usize>,
    tys: Vec<TypeId>,
    // `MetaTable` is invariant over `T`
    marker: PhantomData<Invariant<T>>,
}

impl<T: ?Sized> MetaTable<T> {
    /// Creates a new `MetaTable`.
    pub fn new() -> Self {
        assert_unsized::<T>();

        Default::default()
    }

    /// Registers a resource `R` that implements the trait `T`.
    pub fn register<R>(&mut self)
    where
        R: Resource,
        T: CastFrom<R> + 'static,
    {
        let ty_id = TypeId::of::<R>();
        let vtable_fn = attach_vtable::<T, R>;

        // Important: ensure no entry exists twice!
        let len = self.indices.len();
        match self.indices.entry(ty_id) {
            Entry::Occupied(occ) => {
                let ind = *occ.get();

                self.vtable_fns[ind] = vtable_fn;
            }
            Entry::Vacant(vac) => {
                vac.insert(len);

                self.vtable_fns.push(vtable_fn);
                self.tys.push(ty_id);
            }
        }
    }

    /// Tries to convert `world` to a trait object of type `&T`.
    /// If `world` doesn't have an implementation for `T` (or it wasn't
    /// registered), this will return `None`.
    pub fn get<'a>(&self, res: &'a dyn Resource) -> Option<&'a T> {
        self.indices.get(&res.type_id()).map(|&ind| {
            let vtable_fn = self.vtable_fns[ind];

            let ptr = <*const dyn Resource>::cast::<()>(res).cast_mut();
            let trait_ptr = (vtable_fn)(ptr);
            // SAFETY: We retrieved the `vtable_fn` via TypeId so it will attach
            // a vtable that corresponds with the erased type that the TypeId
            // refers to. `vtable_fn` will also preserve the provenance and
            // address (so we can safely produce a shared reference since we
            // started with one).
            unsafe { &*trait_ptr }
        })
    }

    /// Tries to convert `world` to a trait object of type `&mut T`.
    /// If `world` doesn't have an implementation for `T` (or it wasn't
    /// registered), this will return `None`.
    pub fn get_mut<'a>(&self, res: &'a mut dyn Resource) -> Option<&'a mut T> {
        self.indices.get(&res.type_id()).map(|&ind| {
            let vtable_fn = self.vtable_fns[ind];
            let ptr = <*mut dyn Resource>::cast::<()>(res);
            let trait_ptr = (vtable_fn)(ptr);
            // SAFETY: We retrieved the `vtable_fn` via TypeId so it will attach
            // a vtable that corresponds with the erased type that the TypeId
            // refers to. `vtable_fn` will also preserve the provenance and
            // address (so we can safely produce a mutable reference since we
            // started with one).
            unsafe { &mut *trait_ptr }
        })
    }

    /// Iterates all resources that implement `T` and were registered.
    pub fn iter<'a>(&'a self, res: &'a World) -> MetaIter<'a, T> {
        MetaIter {
            vtable_fns: &self.vtable_fns,
            index: 0,
            world: res,
            tys: &self.tys,
            marker: PhantomData,
        }
    }

    /// Iterates all resources that implement `T` and were registered mutably.
    pub fn iter_mut<'a>(&'a self, res: &'a World) -> MetaIterMut<'a, T> {
        MetaIterMut {
            vtable_fns: &self.vtable_fns,
            index: 0,
            world: res,
            tys: &self.tys,
            marker: PhantomData,
        }
    }
}

impl<T> Default for MetaTable<T>
where
    T: ?Sized,
{
    fn default() -> Self {
        MetaTable {
            vtable_fns: Default::default(),
            indices: Default::default(),
            tys: Default::default(),
            marker: Default::default(),
        }
    }
}

fn assert_unsized<T: ?Sized>() {
    use std::mem::size_of;

    assert_eq!(size_of::<&T>(), 2 * size_of::<usize>());
}

#[cfg(test)]
mod tests {
    use super::*;

    trait Object {
        fn method1(&self) -> i32;

        fn method2(&mut self, x: i32);
    }

    unsafe impl<T> CastFrom<T> for dyn Object
    where
        T: Object + 'static,
    {
        fn cast(t: *mut T) -> *mut Self {
            t
        }
    }

    struct ImplementorA(i32);

    impl Object for ImplementorA {
        fn method1(&self) -> i32 {
            self.0
        }

        fn method2(&mut self, x: i32) {
            self.0 += x;
        }
    }

    struct ImplementorB(i32);

    impl Object for ImplementorB {
        fn method1(&self) -> i32 {
            self.0
        }

        fn method2(&mut self, x: i32) {
            self.0 *= x;
        }
    }

    #[test]
    fn test_iter_all() {
        let mut world = World::empty();

        world.insert(ImplementorA(3));
        world.insert(ImplementorB(1));

        let mut table = MetaTable::<dyn Object>::new();
        table.register::<ImplementorA>();
        table.register::<ImplementorB>();

        {
            let mut iter = table.iter(&world);
            assert_eq!(iter.next().unwrap().method1(), 3);
            assert_eq!(iter.next().unwrap().method1(), 1);
        }

        {
            let mut iter_mut = table.iter_mut(&world);
            let mut obj = iter_mut.next().unwrap();
            obj.method2(3);
            assert_eq!(obj.method1(), 6);
            let mut obj = iter_mut.next().unwrap();
            obj.method2(4);
            assert_eq!(obj.method1(), 4);
        }
    }

    #[test]
    fn test_iter_all_after_removal() {
        let mut world = World::empty();

        world.insert(ImplementorA(3));
        world.insert(ImplementorB(1));

        let mut table = MetaTable::<dyn Object>::new();
        table.register::<ImplementorA>();
        table.register::<ImplementorB>();

        {
            let mut iter = table.iter(&world);
            assert_eq!(iter.next().unwrap().method1(), 3);
            assert_eq!(iter.next().unwrap().method1(), 1);
        }

        world.remove::<ImplementorA>().unwrap();

        {
            let mut iter = table.iter(&world);
            assert_eq!(iter.next().unwrap().method1(), 1);
        }

        world.remove::<ImplementorB>().unwrap();
    }

    struct ImplementorC;

    impl Object for ImplementorC {
        fn method1(&self) -> i32 {
            33
        }

        fn method2(&mut self, _x: i32) {
            unimplemented!()
        }
    }

    struct ImplementorD;

    impl Object for ImplementorD {
        fn method1(&self) -> i32 {
            42
        }

        fn method2(&mut self, _x: i32) {
            unimplemented!()
        }
    }

    #[test]
    fn get() {
        let mut world = World::empty();

        world.insert(ImplementorC);
        world.insert(ImplementorD);

        let mut table = MetaTable::<dyn Object>::new();
        table.register::<ImplementorC>();
        table.register::<ImplementorD>();

        assert_eq!(
            table
                .get(&*world.fetch::<ImplementorC>())
                .unwrap()
                .method1(),
            33
        );
        assert_eq!(
            table
                .get(&*world.fetch::<ImplementorD>())
                .unwrap()
                .method1(),
            42
        );

        // Make sure it fulfills `Resource` requirements
        world.insert(table);
    }
}
