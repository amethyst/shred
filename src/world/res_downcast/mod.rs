//! Code is based on <https://github.com/chris-morgan/mopa>
//! with the macro inlined for `Resource`. License files can be found in the
//! directory of this source file, see COPYRIGHT, LICENSE-APACHE and
//! LICENSE-MIT.

#[cfg(test)]
mod tests;

use std::any::TypeId;

use crate::Resource;

impl dyn Resource {
    /// Returns the boxed value if it is of type `T`, or `Err(Self)` if it
    /// isn't.
    #[inline]
    pub fn downcast<T: Resource>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        if self.is::<T>() {
            // SAFETY: We just checked that the type is `T`.
            unsafe { Ok(self.downcast_unchecked()) }
        } else {
            Err(self)
        }
    }

    /// Returns the boxed value, blindly assuming it to be of type `T`.
    ///
    /// # Safety
    ///
    /// If you are not *absolutely certain* of `T`, you *must not* call this.
    /// Using anything other than the correct type `T` for this `Resource`
    /// will result in UB.
    #[inline]
    pub unsafe fn downcast_unchecked<T: Resource>(self: Box<Self>) -> Box<T> {
        // SAFETY: Caller promises the concrete type is `T`.
        unsafe { Box::from_raw(Box::into_raw(self) as *mut T) }
    }

    /// Returns true if the boxed type is the same as `T`
    #[inline]
    pub fn is<T: Resource>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    #[inline]
    pub fn downcast_ref<T: Resource>(&self) -> Option<&T> {
        if self.is::<T>() {
            // SAFETY: We just checked that the type is `T`.
            unsafe { Some(self.downcast_ref_unchecked()) }
        } else {
            Option::None
        }
    }

    /// Returns a reference to the boxed value, blindly assuming it to be of
    /// type `T`.
    ///
    /// # Safety
    ///
    /// If you are not *absolutely certain* of `T`, you *must not* call this.
    /// Using anything other than the correct type `T` for this `Resource`
    /// will result in UB.
    #[inline]
    pub unsafe fn downcast_ref_unchecked<T: Resource>(&self) -> &T {
        // SAFETY: Caller promises the concrete type is `T`.
        unsafe { &*(self as *const Self as *const T) }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`,
    /// or `None` if it isn't.
    #[inline]
    pub fn downcast_mut<T: Resource>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            // SAFETY: We just checked that the type is `T`.
            unsafe { Some(self.downcast_mut_unchecked()) }
        } else {
            Option::None
        }
    }

    /// Returns a mutable reference to the boxed value, blindly assuming it to
    /// be of type `T`.
    ///
    /// # Safety
    ///
    /// If you are not *absolutely certain* of `T`, you *must not* call this.
    /// Using anything other than the correct type `T` for this `Resource`
    /// will result in UB.
    #[inline]
    pub unsafe fn downcast_mut_unchecked<T: Resource>(&mut self) -> &mut T {
        // SAFETY: Caller promises the concrete type is `T`.
        unsafe { &mut *(self as *mut Self as *mut T) }
    }
}
