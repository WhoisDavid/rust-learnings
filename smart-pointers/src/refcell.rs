use crate::cell::Cell;
use std::cell::UnsafeCell;

#[derive(Clone, Copy)]
enum RefState {
    Unshared,      // No reference
    Shared(usize), // &T
    Exclusive,     // &mut T
}
// RefCell is there to do "manual" borrow checking
pub struct RefCell<T> {
    value: UnsafeCell<T>,
    // # of references to the value
    state: Cell<RefState>,
}

impl<T> RefCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            state: Cell::new(RefState::Unshared),
        }
    }

    pub fn borrow(&self) -> Option<Ref<'_, T>> {
        match self.state.get() {
            RefState::Unshared => {
                self.state.set(RefState::Shared(1));
                // SAFETY: no exclusive references given out since state would be Exclusive
                Some(Ref { refcell: self })
            }
            RefState::Shared(n) => {
                self.state.set(RefState::Shared(n + 1));
                // SAFETY: no exclusive references given out since state would be Exclusive
                Some(Ref { refcell: self })
            }
            RefState::Exclusive => None,
        }
    }

    pub fn borrow_mut(&self) -> Option<RefMut<'_, T>> {
        if let RefState::Unshared = self.state.get() {
            self.state.set(RefState::Exclusive);
            // SAFETY: no other references given out since state would be Shared or Exclusive
            Some(RefMut { refcell: self })
        } else {
            None
        }
    }
}

pub struct Ref<'refcell, T> {
    refcell: &'refcell RefCell<T>,
}

impl<T> std::ops::Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: a Ref is only created if state is not Exclusive
        // and the state is changed to Shared when it is created
        // so dereferencing into a shared reference is fine
        unsafe { &*self.refcell.value.get() }
    }
}

impl<T> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        let new_state = match self.refcell.state.get() {
            RefState::Shared(1) => RefState::Unshared,
            RefState::Shared(n) => RefState::Shared(n - 1),
            _ => unreachable!(),
        };
        self.refcell.state.set(new_state);
    }
}

pub struct RefMut<'refcell, T> {
    refcell: &'refcell RefCell<T>,
}

impl<T> std::ops::Deref for RefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY
        // See Safety for DerefMut
        unsafe { &*self.refcell.value.get() }
    }
}

impl<T> std::ops::DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY
        // a RefMut is only created if no other reference exist and the state is set to Exclusive
        // so no future references will be given out/
        // Exclusive lease on the inner value, so mutably dereferencing is fine
        unsafe { &mut *self.refcell.value.get() }
    }
}

impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        let new_state = match self.refcell.state.get() {
            RefState::Exclusive => RefState::Unshared,
            _ => unreachable!(),
        };
        self.refcell.state.set(new_state);
    }
}
