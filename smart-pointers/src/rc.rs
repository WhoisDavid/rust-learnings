use crate::cell::Cell;
use std::{marker::PhantomData, ptr::NonNull};
// Rc is great for single-threaded applications where you need to keep multiple reference to an object.const
// E.g GUI loops, big binary you do not want to copy, etc...
struct RcInner<T> {
    value: T,
    refcount: Cell<usize>,
}

pub struct Rc<T> {
    inner: NonNull<RcInner<T>>,
    // *mut and *const are raw pointers - no guarantee on shared refs/exclusivity, requires unsafe code
    // NonNull = *mut T but non-zero and covariant - must always be non-null, used mainly for compiler optmization
    _marker: PhantomData<RcInner<T>>,
}

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        let inner = Box::new(RcInner {
            value,
            refcount: Cell::new(1),
        });
        Self {
            // SAFETY: Box does not give a null pointer
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(inner)) },
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        let inner = unsafe { self.inner.as_ref() };
        let rc = inner.refcount.get();
        inner.refcount.set(rc + 1);
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<T> std::ops::Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY
        // self.inner is a Box that is only deallocated when the last Rc goes away
        // We have an Rc, therefore the Box has not been deallocated, so deref is fine
        &unsafe { self.inner.as_ref() }.value
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.inner.as_ref() };
        let rc = inner.refcount.get();
        if rc == 1 {
            // Drop the final pointer
            drop(inner);
            // SAFETY: this was the only reference left so we drop the inner value (box)
            let _ = unsafe { Box::from_raw(self.inner.as_ptr()) };
        } else {
            // There are other Rcs so no dropping
            inner.refcount.set(rc - 1)
        }
    }
}

#[cfg(test)]
mod test {
    // use super::*;
    // #[test]
    // fn bad() {
    //     let (y, x);
    //     x = String::from("hello");
    //     y = Rc::new(&x)
    // }
}
