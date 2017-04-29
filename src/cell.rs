use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
#[cfg(debug_assertions)]
use std::sync::Arc;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Ref<'a, T: 'a> {
    #[cfg(debug_assertions)]
    flag: Arc<AtomicUsize>,
    value: &'a T,
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

#[cfg(debug_assertions)]
impl<'a, T> Drop for Ref<'a, T> {
    fn drop(&mut self) {
        self.flag.fetch_sub(1, Ordering::Release);
    }
}

pub struct RefMut<'a, T: 'a> {
    #[cfg(debug_assertions)]
    flag: Arc<AtomicUsize>,
    value: &'a mut T,
}

impl<'a, T> Deref for RefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T> DerefMut for RefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}

#[cfg(debug_assertions)]
impl<'a, T> Drop for RefMut<'a, T> {
    fn drop(&mut self) {
        self.flag.store(0, Ordering::Release)
    }
}

/// A custom cell similar to
/// `RefCell`, but
///
/// 1) only checks rules in debug mode
/// 2) is thread-safe
pub struct TrustCell<T> {
    #[cfg(debug_assertions)]
    flag: Arc<AtomicUsize>,
    inner: UnsafeCell<T>,
}

impl<T> TrustCell<T> {
    #[cfg(not(debug_assertions))]
    pub fn new(val: T) -> Self {
        TrustCell { inner: UnsafeCell::new(val) }
    }

    #[cfg(debug_assertions)]
    pub fn new(val: T) -> Self {
        TrustCell {
            flag: Arc::new(AtomicUsize::new(0)),
            inner: UnsafeCell::new(val),
        }
    }

    #[cfg(not(debug_assertions))]
    pub unsafe fn borrow(&self) -> Ref<T> {
        Ref { value: &*self.inner.get() }
    }

    #[cfg(debug_assertions)]
    pub unsafe fn borrow(&self) -> Ref<T> {
        debug_assert!(self.flag.load(Ordering::Acquire) != !0,
                      "already borrowed mutably");

        self.flag.fetch_add(1, Ordering::Release);

        Ref {
            flag: self.flag.clone(),
            value: &*self.inner.get(),
        }
    }

    #[cfg(not(debug_assertions))]
    pub unsafe fn borrow_mut(&self) -> RefMut<T> {
        RefMut { value: &mut *self.inner.get() }
    }

    #[cfg(debug_assertions)]
    pub unsafe fn borrow_mut(&self) -> RefMut<T> {
        debug_assert!(self.flag.load(Ordering::Acquire) == 0, "already borrowed");

        self.flag.store(!0, Ordering::Release);

        RefMut {
            flag: self.flag.clone(),
            value: &mut *self.inner.get(),
        }
    }
}

unsafe impl<T> Sync for TrustCell<T> where T: Sync {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi() {
        let cell: TrustCell<_> = TrustCell::new(5);

        unsafe {
            let a = cell.borrow();
            let b = cell.borrow();

            assert_eq!(10, *a + *b);
        }
    }

    #[test]
    fn write() {
        let cell: TrustCell<_> = TrustCell::new(5);

        unsafe {
            let mut a = cell.borrow_mut();
            *a += 2;
            *a += 3;
        }

        unsafe { assert_eq!(10, *cell.borrow()); }
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "already borrowed mutably")]
    fn panic_already() {
        let cell: TrustCell<_> = TrustCell::new(5);

        unsafe {
            let mut a = cell.borrow_mut();
            *a = 7;

            assert_eq!(7, *cell.borrow());
        }
    }
}
