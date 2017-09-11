use std::thread::{ThreadId, current as current_thread};

/// A wrapper for a thread local resource.
/// This struct only gives access to the wrapped data
/// if the accessing thread is the one this resource has been created in.
///
/// Do not send this struct to another thread and drop it there,
/// otherwise you will get UB.
///
/// **Note:** This is essentially a hack and is **not** verified
/// by Rust's type system.
pub struct ThreadLocal<T> {
    value: T,
    thread_id: ThreadId,
}

impl<T> ThreadLocal<T> {
    /// Creates a new thread-local resource.
    pub fn new(value: T) -> Self {
        ThreadLocal {
            value,
            thread_id: current_id(),
        }
    }
}

impl<T> ThreadLocal<T> {
    /// Tries to retrieve the wrapped data.
    /// Returns `None` if this thraed is not allowed to access
    /// the data.
    pub fn get(&self) -> Option<&T> {
        if self.thread_id == current_id() {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Returns a reference to the wrapped data without
    /// checking if this thread is allowed to do that.
    pub unsafe fn get_unchecked(&self) -> &T {
        &self.value
    }

    /// Tries to retrieve the wrapped data mutably.
    /// Returns `None` if this thraed is not allowed to access
    /// the data.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.thread_id == current_id() {
            Some(&mut self.value)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the wrapped data without
    /// checking if this thread is allowed to do that.
    pub unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

unsafe impl<T> Send for ThreadLocal<T> {}
unsafe impl<T> Sync for ThreadLocal<T> {}

fn current_id() -> ThreadId {
    current_thread().id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_check() {
        let local = ThreadLocal::new(5);

        assert!(local.get().is_some());

        ::std::thread::spawn(move || assert!(local.get().is_none()))
            .join()
            .expect("Expected None, but got access to the resource")
    }
}
