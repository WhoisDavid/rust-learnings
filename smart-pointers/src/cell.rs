use std::cell::UnsafeCell;

pub struct Cell<T> {
    value: UnsafeCell<T>,
}

impl<T> Cell<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn set(&self, value: T) {
        // SAFETY: no-one else is concurrently mutating self.value (because !Sync)
        // SAFETY: we are not invalidating any references because we never give any out
        unsafe { *self.value.get() = value }
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        // SAFETY: no-one else is modifying this valus since only this thread can mutate
        // (because !Sync) and it is execuitng tis function instead
        unsafe { *self.value.get() }
    }
}

// To test failure with Sync
// unsafe impl<T> Sync for Cell<T> {}

#[cfg(test)]
mod test {
    // use super::Cell;
    // #[test]
    // fn bad_if_sync() {
    //     use std::sync::Arc;
    //     let x = Arc::new(Cell::new(0));
    //     let x1 = Arc::clone(&x);
    //     let y1 = std::thread::spawn(move || {
    //         (0..1000).for_each(|_| {
    //             let x = x1.get();
    //             x1.set(x + 1);
    //         });
    //     });
    //     let x2 = Arc::clone(&x);
    //     let y2 = std::thread::spawn(move || {
    //         (0..1000).for_each(|_| {
    //             let x = x2.get();
    //             x2.set(x + 1);
    //         });
    //     });
    //     y1.join().unwrap();
    //     y2.join().unwrap();
    //     // eprintln!("{}", x.get());
    //     assert_eq!(x.get(), 2000)
    // }

    // #[test]
    // fn bad2() {
    //     let x = Cell::new(String::from("hello"));
    //     let first = x.get();
    //     x.set(String::from("world"));
    //     eprintln!("{}", first)
    // }
}
