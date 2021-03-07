use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
/// Different flavors of channels:
/// - Synchronous channels: Channel where send() can block. Limited capacity.
///   - Mutex + Condvar + VecDeque
///   - Atomic VecDeque (atomic queue) + thread::park + thread::Thread::notify
/// - Asynchronous channels: Channel where send() cannot block. Unbounded.
///   - Mutex + Condvar + VecDeque
///   - Mutex + Condvar + LinkedList
///   - Atomic linked list, linked list of T
///   - Atomic block linked list, linked list of atomic VecDeque<T>
/// - Rendezvous channels: Synchronous with capacity = 0. Used for thread synchronization.
/// - Oneshot channels: Any capacity. In practice, only one call to send().

#[derive(Default)]
struct Inner<T> {
    queue: VecDeque<T>,
    senders: usize,
}
struct Shared<T> {
    inner: Mutex<Inner<T>>,
    available: Condvar,
}
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    pub fn send(&mut self, t: T) {
        // Acquire lock
        let mut inner = self.shared.inner.lock().unwrap();
        inner.queue.push_back(t);
        // Release lock
        drop(inner);
        // Notify blocked thread
        self.shared.available.notify_one();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        // Acquire lock
        let mut inner = self.shared.inner.lock().unwrap();
        // Increase senders counter
        inner.senders += 1;
        // Release lock
        drop(inner);
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // Acquire lock
        let mut inner = self.shared.inner.lock().unwrap();
        // Decrease senders counter
        inner.senders -= 1;
        let was_last = inner.senders == 0;
        // Release lock
        drop(inner);

        // If it was the last sender, notify the receiver thread
        // so that it doesn't hang waiting for a sender that does not exists
        if was_last {
            self.shared.available.notify_one()
        }
    }
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
    buffer: VecDeque<T>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Option<T> {
        if !self.buffer.is_empty() {
            return self.buffer.pop_front();
        }
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            match inner.queue.pop_front() {
                Some(t) => {
                    std::mem::swap(&mut self.buffer, &mut inner.queue);
                    return Some(t);
                }
                None if inner.senders == 0 => return None,
                None => inner = self.shared.available.wait(inner).unwrap(),
            }
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Inner {
        queue: VecDeque::new(),
        senders: 1,
    };
    let shared = Shared {
        inner: Mutex::new(inner),
        available: Condvar::new(),
    };
    let shared = Arc::new(shared);
    let tx = Sender {
        shared: Arc::clone(&shared),
    };
    let rx = Receiver {
        shared: Arc::clone(&shared),
        buffer: VecDeque::default(),
    };
    (tx, rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ping_pong() {
        let (mut tx, mut rx) = channel();
        tx.send(42);
        tx.send(7);
        tx.send(12);
        assert_eq!(rx.recv(), Some(42));
        assert_eq!(rx.recv(), Some(7));
        assert_eq!(rx.recv(), Some(12));
    }

    #[test]
    fn closed_tx() {
        let (tx, mut rx) = channel::<()>();
        // Drop the only sender
        drop(tx);
        assert_eq!(rx.recv(), None)
    }

    #[test]
    fn closed_rx() {
        let (mut tx, rx) = channel();
        // Drop the only receiver
        drop(rx);
        tx.send(12)
    }
}
