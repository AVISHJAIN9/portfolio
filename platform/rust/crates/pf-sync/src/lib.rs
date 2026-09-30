use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Sequential Lock (Seqlock) for high-frequency reads with occasional atomic writes.
/// Readers never block writers and retry if a concurrent write occurs.
pub struct SeqLock<T> {
    sequence: AtomicUsize,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SeqLock<T> {}
unsafe impl<T: Send> Sync for SeqLock<T> {}

impl<T: Copy> SeqLock<T> {
    pub const fn new(val: T) -> Self {
        Self {
            sequence: AtomicUsize::new(0),
            data: UnsafeCell::new(val),
        }
    }

    /// Read data with optimistic concurrency retry loop.
    pub fn read(&self) -> T {
        loop {
            let seq0 = self.sequence.load(Ordering::Acquire);
            if seq0 & 1 != 0 {
                std::hint::spin_loop();
                continue;
            }
            let val = unsafe { *self.data.get() };
            let seq1 = self.sequence.load(Ordering::Acquire);
            if seq0 == seq1 {
                return val;
            }
            std::hint::spin_loop();
        }
    }

    /// Atomic write updating data and sequence counters.
    pub fn write(&self, val: T) {
        let seq = self.sequence.fetch_add(1, Ordering::AcqRel);
        unsafe { *self.data.get() = val };
        self.sequence.store(seq + 2, Ordering::Release);
    }
}

/// SPSC Lock-Free Bounded Ring Buffer Queue.
pub struct SpscQueue<T, const N: usize> {
    buffer: Vec<parking_lot::Mutex<Option<T>>>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T, const N: usize> Default for SpscQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> SpscQueue<T, N> {
    pub fn new() -> Self {
        assert!(N > 0, "Capacity must be positive");
        let mut buffer = Vec::with_capacity(N);
        for _ in 0..N {
            buffer.push(parking_lot::Mutex::new(None));
        }
        Self {
            buffer,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, item: T) -> Result<(), T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail - head >= N {
            return Err(item); // Queue full
        }
        *self.buffer[tail % N].lock() = Some(item);
        self.tail.store(tail + 1, Ordering::Release);
        Ok(())
    }

    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head == tail {
            return None; // Queue empty
        }
        let item = self.buffer[head % N].lock().take();
        self.head.store(head + 1, Ordering::Release);
        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seqlock() {
        let lock = SeqLock::new(42u64);
        assert_eq!(lock.read(), 42);
        lock.write(100);
        assert_eq!(lock.read(), 100);
    }

    #[test]
    fn test_spsc_queue() {
        let queue = SpscQueue::<i32, 4>::new();
        assert!(queue.push(1).is_ok());
        assert!(queue.push(2).is_ok());
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), None);
    }
}
