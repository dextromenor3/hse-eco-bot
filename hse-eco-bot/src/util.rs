use std::ops::Deref;
use std::rc::Rc;

pub struct UnsafeRc<T> {
    inner: Rc<T>,
}

impl<T> UnsafeRc<T> {
    /// SAFETY: the caller must ensure that, at each moment of time,
    /// the created `UnsafeRc` and all its clones belong to at most one thread.
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: Rc::new(value),
        }
    }
}

impl<T> Deref for UnsafeRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Clone for UnsafeRc<T> {
    fn clone(&self) -> Self {
        Self { inner: Rc::clone(&self.inner) }
    }
}

unsafe impl<T> Send for UnsafeRc<T> {}
