//! Helper module for some internals, most users don't need to interact with it.

use std::cmp;
use std::hash;
use std::cell::UnsafeCell;
use std::error::Error;
use std::fmt::{Display, Error as FormatError, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::usize;

/// Marker struct for an invalid borrow error
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct InvalidBorrow;

impl Display for InvalidBorrow {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FormatError> {
        write!(f, "Tried to borrow when it was illegal")
    }
}

impl Error for InvalidBorrow {
    fn description(&self) -> &str {
        "This error is returned when you try to borrow immutably when it's already \
         borrowed mutably or you try to borrow mutably when it's already borrowed"
    }
}

/// An immutable reference to data in a `TrustCell`.
///
/// Access the value via `std::ops::Deref` (e.g. `*val`)
#[derive(Debug)]
pub struct Ref<'a, T: 'a> {
    flag: &'a AtomicUsize,
    value: &'a T,
}

impl<'a, T> PartialEq for Ref<'a, T>
    where T: PartialEq
{
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(other.value)
    }
}

impl<'a, T> Eq for Ref<'a, T> where T: Eq {}

impl<'a, T> PartialOrd for Ref<'a, T>
    where T: PartialOrd
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.value.partial_cmp(other.value)
    }
}

impl<'a, T> Ord for Ref<'a, T>
    where T: Ord
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.value.cmp(other.value)
    }
}

impl<'a, T> hash::Hash for Ref<'a, T>
    where T: hash::Hash
{
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T> Drop for Ref<'a, T> {
    fn drop(&mut self) {
        self.flag.fetch_sub(1, Ordering::Release);
    }
}

/// A mutable reference to data in a `TrustCell`.
///
/// Access the value via `std::ops::DerefMut` (e.g. `*val`)
#[derive(Debug)]
pub struct RefMut<'a, T: 'a> {
    flag: &'a AtomicUsize,
    value: &'a mut T,
}

impl<'a, T> PartialEq for RefMut<'a, T>
    where T: PartialEq
{
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<'a, T> Eq for RefMut<'a, T> where T: Eq {}

impl<'a, T> PartialOrd for RefMut<'a, T>
    where T: PartialOrd
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<'a, T> Ord for RefMut<'a, T>
    where T: Ord
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<'a, T> hash::Hash for RefMut<'a, T>
    where T: hash::Hash
{
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
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

impl<'a, T> Drop for RefMut<'a, T> {
    fn drop(&mut self) {
        self.flag.store(0, Ordering::Release)
    }
}

/// A custom cell container that is a `RefCell` with thread-safety.
#[derive(Debug)]
pub struct TrustCell<T> {
    flag: AtomicUsize,
    inner: UnsafeCell<T>,
}

impl<T> TrustCell<T> {
    /// Create a new cell, similar to `RefCell::new`
    pub fn new(val: T) -> Self {
        TrustCell {
            flag: AtomicUsize::new(0),
            inner: UnsafeCell::new(val),
        }
    }

    /// Get an immutable reference to the inner data.
    ///
    /// Absence of write accesses is checked at run-time.
    ///
    /// # Panics
    ///
    /// This function will panic if there is a mutable reference to the data already in use.
    pub fn borrow(&self) -> Ref<T> {
        self.check_flag_read().expect("Already borrowed mutably");

        Ref {
            flag: &self.flag,
            value: unsafe { &*self.inner.get() },
        }
    }

    /// Get an immutable reference to the inner data.
    ///
    /// Absence of write accesses is checked at run-time. If access is not possible, an error is
    /// returned.
    pub fn try_borrow(&self) -> Result<Ref<T>, InvalidBorrow> {
        self.check_flag_read()?;

        Ok(Ref {
            flag: &self.flag,
            value: unsafe { &*self.inner.get() },
        })
    }

    /// Get a mutable reference to the inner data.
    ///
    /// Exclusive access is checked at run-time.
    ///
    /// # Panics
    ///
    /// This function will panic if there are any references to the data already in use.
    pub fn borrow_mut(&self) -> RefMut<T> {
        self.check_flag_write().expect("Already borrowed");

        RefMut {
            flag: &self.flag,
            value: unsafe { &mut *self.inner.get() },
        }
    }

    /// Get a mutable reference to the inner data.
    ///
    /// Exclusive access is checked at run-time. If access is not possible, an error is returned.
    pub fn try_borrow_mut(&self) -> Result<RefMut<T>, InvalidBorrow> {
        self.check_flag_write()?;

        Ok(RefMut {
            flag: &self.flag,
            value: unsafe { &mut *self.inner.get() },
        })
    }

    /// Gets exclusive access to the inner value, bypassing the Cell.
    ///
    /// Exclusive access is checked at compile time.
    pub fn get_mut(&mut self) -> &mut T {
        // safe because we have exclusive access via &mut self
        unsafe { &mut *self.inner.get() }
    }

    /// Make sure we are allowed to aquire a read lock, and increment the read count by 1
    fn check_flag_read(&self) -> Result<(), InvalidBorrow> {
        // Check that no write reference is out, then try to increment the read count and return
        // once successful.
        loop {
            let val = self.flag.load(Ordering::Acquire);

            if val == usize::MAX {
                return Err(InvalidBorrow);
            }

            if self.flag.compare_and_swap(val, val + 1, Ordering::AcqRel) == val {
                return Ok(());
            }
        }
    }

    /// Make sure we are allowed to aquire a write lock, and then set the write lock flag.
    fn check_flag_write(&self) -> Result<(), InvalidBorrow> {
        // Check we have 0 references out, and then set the ref count to usize::MAX to
        // indicate a write lock.
        match self.flag.compare_and_swap(0, usize::MAX, Ordering::AcqRel) {
            0 => Ok(()),
            _ => Err(InvalidBorrow),
        }
    }
}

unsafe impl<T> Sync for TrustCell<T>
where
    T: Sync,
{
}

impl<T> Default for TrustCell<T>
where
    T: Default,
{
    fn default() -> Self {
        TrustCell::new(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_multiple_reads() {
        let cell: TrustCell<_> = TrustCell::new(5);

        let a = cell.borrow();
        let b = cell.borrow();

        assert_eq!(10, *a + *b);
    }

    #[test]
    fn allow_single_write() {
        let cell: TrustCell<_> = TrustCell::new(5);

        {
            let mut a = cell.borrow_mut();
            *a += 2;
            *a += 3;
        }

        assert_eq!(10, *cell.borrow());
    }

    #[test]
    #[should_panic(expected = "Already borrowed mutably")]
    fn panic_write_and_read() {
        let cell: TrustCell<_> = TrustCell::new(5);

        let mut a = cell.borrow_mut();
        *a = 7;

        assert_eq!(7, *cell.borrow());
    }

    #[test]
    #[should_panic(expected = "Already borrowed")]
    fn panic_write_and_write() {
        let cell: TrustCell<_> = TrustCell::new(5);

        let mut a = cell.borrow_mut();
        *a = 7;

        assert_eq!(7, *cell.borrow_mut());
    }

    #[test]
    #[should_panic(expected = "Already borrowed")]
    fn panic_read_and_write() {
        let cell: TrustCell<_> = TrustCell::new(5);

        let _a = cell.borrow();

        assert_eq!(7, *cell.borrow_mut());
    }

    #[test]
    fn try_write_and_read() {
        let cell: TrustCell<_> = TrustCell::new(5);

        let mut a = cell.try_borrow_mut().unwrap();
        *a = 7;

        assert_eq!(Err(InvalidBorrow), cell.try_borrow());
    }

    #[test]
    fn try_write_and_write() {
        let cell: TrustCell<_> = TrustCell::new(5);

        let mut a = cell.try_borrow_mut().unwrap();
        *a = 7;

        assert_eq!(Err(InvalidBorrow), cell.try_borrow_mut());
    }

    #[test]
    fn try_read_and_write() {
        let cell: TrustCell<_> = TrustCell::new(5);

        let _a = cell.try_borrow().unwrap();

        assert_eq!(Err(InvalidBorrow), cell.try_borrow_mut());
    }
}
