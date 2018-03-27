use {Resource, Resources};
use fxhash::FxHashMap;
use mopa::Any;
use std::any::TypeId;

/// Helper trait for the `MetaTable`.
/// This trait is required to be implemented for a trait to be compatible with the meta table.
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
/// impl<T> CastFrom<T> for Foo
/// where
///     T: Foo + 'static,
/// {
///     fn cast(t: &T) -> &(Foo + 'static) {
///         t
///     }
///
///     fn cast_mut(t: &mut T) -> &mut (Foo + 'static) {
///         t
///     }
/// }
/// ```
pub trait CastFrom<T> {
    /// Casts an immutable `T` reference to a trait object.
    fn cast(t: &T) -> &Self;

    /// Casts a mutable `T` reference to a trait object.
    fn cast_mut(t: &mut T) -> &mut Self;
}

/// An iterator for the `MetaTable`.
pub struct MetaIter<'a, T: ?Sized + 'a> {
    fat: &'a [unsafe fn(&Resource) -> &T],
    index: usize,
    res: &'a mut Resources,
    tys: &'a [TypeId],
}

impl<'a, T> Iterator for MetaIter<'a, T>
where
    T: ?Sized + 'a,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        use std::mem::transmute;

        let index = self.index;
        self.index += 1;

        // Ugly hack that works due to `UnsafeCell` and distinct resources.
        unsafe {
            transmute::<&mut Resources, &'a mut Resources>(&mut self.res)
                .get_mut_raw(match self.tys.get(index) {
                    Some(&x) => x,
                    None => return None,
                })
                .map(|res| (self.fat[index])(&*res))
                .or_else(|| self.next())
        }
    }
}

/// A mutable iterator for the `MetaTable`.
pub struct MetaIterMut<'a, T: ?Sized + 'a> {
    fat_mut: &'a [unsafe fn(&mut Resource) -> &mut T],
    index: usize,
    res: &'a mut Resources,
    tys: &'a [TypeId],
}

impl<'a, T> Iterator for MetaIterMut<'a, T>
where
    T: ?Sized + 'a,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        use std::mem::transmute;

        let index = self.index;
        self.index += 1;

        // Ugly hack that works due to `UnsafeCell` and distinct resources.
        unsafe {
            transmute::<&mut Resources, &'a mut Resources>(&mut self.res)
                .get_mut_raw(match self.tys.get(index) {
                    Some(&x) => x,
                    None => return None,
                })
                .map(|res| (self.fat_mut[index])(res))
                .or_else(|| self.next())
        }
    }
}

/// The `MetaTable` which allows to store object-safe trait implementations for resources.
///
/// For example, you have a trait `Foo` that is implemented by several resources.
/// You can register all the implementors using `MetaTable::register`. Later on, you
/// can iterate over all resources that implement `Foo` without knowing their specific type.
///
/// # Examples
///
/// ```
/// use shred::{CastFrom, MetaTable, Resources};
///
/// trait Object {
///     fn method1(&self) -> i32;
///
///     fn method2(&mut self, x: i32);
/// }
///
/// impl<T> CastFrom<T> for Object
/// where
///     T: Object + 'static,
/// {
///     fn cast(t: &T) -> &Self {
///         t
///     }
///
///     fn cast_mut(t: &mut T) -> &mut Self {
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
/// let mut res = Resources::new();
///
/// res.insert(ImplementorA(3));
/// res.insert(ImplementorB(1));
///
/// let mut table = MetaTable::<Object>::new();
/// table.register::<ImplementorA>();
/// table.register::<ImplementorB>();
///
/// {
///     let mut iter = table.iter(&mut res);
///     assert_eq!(iter.next().unwrap().method1(), 3);
///     assert_eq!(iter.next().unwrap().method1(), 1);
/// }
/// ```
pub struct MetaTable<T: ?Sized> {
    fat: Vec<unsafe fn(&Resource) -> &T>,
    fat_mut: Vec<unsafe fn(&mut Resource) -> &mut T>,
    indices: FxHashMap<TypeId, usize>,
    tys: Vec<TypeId>,
}

impl<T: ?Sized> MetaTable<T> {
    /// Creates a new `MetaTable`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Registers a resource `R` that implements the trait `T`.
    pub fn register<R>(&mut self)
    where
        R: Resource,
        T: CastFrom<R> + 'static,
    {
        use std::collections::hash_map::Entry;

        unsafe fn fat<R, T>(r: &Resource) -> &T
        where
            R: Resource,
            T: CastFrom<R> + ?Sized + 'static,
        {
            let r: &R = r.downcast_ref_unchecked();

            T::cast(r)
        }

        unsafe fn fat_mut<R, T>(r: &mut Resource) -> &mut T
        where
            R: Resource,
            T: CastFrom<R> + ?Sized + 'static,
        {
            let r: &mut R = r.downcast_mut_unchecked();

            T::cast_mut(r)
        }

        let fat = fat::<R, T>;
        let fat_mut = fat_mut::<R, T>;

        let ty_id = TypeId::of::<R>();

        // Important: ensure no entry exists twice!
        let len = self.indices.len();
        match self.indices.entry(ty_id) {
            Entry::Occupied(occ) => {
                let ind = *occ.get();

                self.fat[ind] = fat;
                self.fat_mut[ind] = fat_mut;
            }
            Entry::Vacant(vac) => {
                vac.insert(len);

                self.fat.push(fat);
                self.fat_mut.push(fat_mut);
                self.tys.push(ty_id);
            }
        }
    }

    /// Tries to convert `res` to a trait object of type `&T`.
    /// If `res` doesn't have an implementation for `T` (or it wasn't registered),
    /// this will return `None`.
    pub fn get<'a>(&self, res: &'a Resource) -> Option<&'a T> {
        unsafe {
            self.indices
                .get(&Any::get_type_id(res))
                .map(move |&ind| (self.fat[ind])(res))
        }
    }

    /// Tries to convert `res` to a trait object of type `&mut T`.
    /// If `res` doesn't have an implementation for `T` (or it wasn't registered),
    /// this will return `None`.
    pub fn get_mut<'a>(&self, res: &'a mut Resource) -> Option<&'a mut T> {
        unsafe {
            self.indices
                .get(&Any::get_type_id(res))
                .map(move |&ind| (self.fat_mut[ind])(res))
        }
    }

    /// Iterates all resources that implement `T` and were registered.
    pub fn iter<'a>(&'a self, res: &'a mut Resources) -> MetaIter<'a, T> {
        MetaIter {
            fat: &self.fat,
            index: 0,
            res,
            tys: &self.tys,
        }
    }

    /// Iterates all resources that implement `T` and were registered mutably.
    pub fn iter_mut<'a>(&'a self, res: &'a mut Resources) -> MetaIterMut<'a, T> {
        MetaIterMut {
            fat_mut: &self.fat_mut,
            index: 0,
            res,
            tys: &self.tys,
        }
    }
}

impl<T> Default for MetaTable<T>
where
    T: ?Sized,
{
    fn default() -> Self {
        MetaTable {
            fat: vec![],
            fat_mut: vec![],
            indices: Default::default(),
            tys: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Resources;

    trait Object {
        fn method1(&self) -> i32;

        fn method2(&mut self, x: i32);
    }

    impl<T> CastFrom<T> for Object
    where
        T: Object + 'static,
    {
        fn cast(t: &T) -> &Self {
            t
        }

        fn cast_mut(t: &mut T) -> &mut Self {
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
        let mut res = Resources::new();

        res.insert(ImplementorA(3));
        res.insert(ImplementorB(1));

        let mut table = MetaTable::<Object>::new();
        table.register::<ImplementorA>();
        table.register::<ImplementorB>();

        {
            let mut iter = table.iter(&mut res);
            assert_eq!(iter.next().unwrap().method1(), 3);
            assert_eq!(iter.next().unwrap().method1(), 1);
        }

        {
            let mut iter_mut = table.iter_mut(&mut res);
            let obj = iter_mut.next().unwrap();
            obj.method2(3);
            assert_eq!(obj.method1(), 6);
            let obj = iter_mut.next().unwrap();
            obj.method2(4);
            assert_eq!(obj.method1(), 4);
        }
    }
}
