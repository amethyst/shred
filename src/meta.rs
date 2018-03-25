use std::any::TypeId;

use fxhash::FxHashMap;
use mopa::Any;

use {Resource, Resources};

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
                .get_mut_raw(*self.tys.get(index)?)
                .map(|res| (self.fat[index])(&*res))
                .or_else(|| self.next())
        }
    }
}

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
                .get_mut_raw(*self.tys.get(index)?)
                .map(|res| (self.fat_mut[index])(res))
                .or_else(|| self.next())
        }
    }
}

pub struct MetaTable<T: ?Sized> {
    fat: Vec<unsafe fn(&Resource) -> &T>,
    fat_mut: Vec<unsafe fn(&mut Resource) -> &mut T>,
    indices: FxHashMap<TypeId, usize>,
    tys: Vec<TypeId>,
}

impl<T: ?Sized> MetaTable<T> {
    pub fn new() -> Self {
        MetaTable {
            fat: vec![],
            fat_mut: vec![],
            indices: Default::default(),
            tys: vec![],
        }
    }

    pub fn insert<R: Resource>(
        &mut self,
        fat: unsafe fn(&Resource) -> &T,
        fat_mut: unsafe fn(&mut Resource) -> &mut T,
    ) {
        // Important: ensure no entry exists twice!

        use std::collections::hash_map::Entry;

        let ty_id = TypeId::of::<R>();

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

    pub fn get<'a>(&self, res: &'a Resource) -> Option<&'a T> {
        unsafe {
            self.indices
                .get(&Any::get_type_id(res))
                .map(move |&ind| (self.fat[ind])(res))
        }
    }

    pub fn get_mut<'a>(&self, res: &'a mut Resource) -> Option<&'a mut T> {
        unsafe {
            self.indices
                .get(&Any::get_type_id(res))
                .map(move |&ind| (self.fat_mut[ind])(res))
        }
    }

    pub fn iter<'a>(&'a self, res: &'a mut Resources) -> MetaIter<'a, T> {
        MetaIter {
            fat: &self.fat,
            index: 0,
            res,
            tys: &self.tys,
        }
    }

    pub fn iter_mut<'a>(&'a self, res: &'a mut Resources) -> MetaIterMut<'a, T> {
        MetaIterMut {
            fat_mut: &self.fat_mut,
            index: 0,
            res,
            tys: &self.tys,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use {Resource, Resources};

    trait Object {
        fn method1(&self) -> i32;

        fn method2(&mut self, x: i32);
    }

    unsafe fn object_provider<T: Object + Resource + 'static>(t: &Resource) -> &(Object + 'static) {
        let t: &T = t.downcast_ref_unchecked();

        t as &Object
    }

    unsafe fn object_provider_mut<T: Object + Resource + 'static>(
        t: &mut Resource,
    ) -> &mut (Object + 'static) {
        let t: &mut T = t.downcast_mut_unchecked();

        t as &mut Object
    }

    fn reg_object<T: Object + Resource>(table: &mut MetaTable<Object>) {
        table.insert::<T>(object_provider::<T>, object_provider_mut::<T>)
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

        let mut table = MetaTable::new();

        reg_object::<ImplementorA>(&mut table);
        reg_object::<ImplementorB>(&mut table);

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
