use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// Generic cache interface.
pub trait Cache<K, V> {
    fn get(&mut self, key: &K) -> Option<&V>;
    fn insert(&mut self, key: K, value: V);
    fn remove(&mut self, key: &K) -> Option<V>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// S3-FIFO (Simple, Scalable, Small FIFO) Cache Eviction Policy (SOSP '23).
/// Uses 3 FIFO queues: Small (S), Main (M), and Ghost (G) to achieve superior hit rates with minimal lock contention.
pub struct S3FifoCache<K: Clone + Eq + Hash, V> {
    capacity: usize,
    small_cap: usize,
    ghost_cap: usize,

    entries: HashMap<K, (V, u8)>, // (Value, frequency/hit count 0..=3)
    small_queue: VecDeque<K>,
    main_queue: VecDeque<K>,
    ghost_queue: VecDeque<K>,
    ghost_set: HashMap<K, ()>,
}

impl<K: Clone + Eq + Hash, V> S3FifoCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        let capacity = std::cmp::max(4, capacity);
        let small_cap = std::cmp::max(1, capacity / 10);
        let ghost_cap = capacity;

        Self {
            capacity,
            small_cap,
            ghost_cap,
            entries: HashMap::with_capacity(capacity),
            small_queue: VecDeque::with_capacity(small_cap),
            main_queue: VecDeque::with_capacity(capacity),
            ghost_queue: VecDeque::with_capacity(ghost_cap),
            ghost_set: HashMap::with_capacity(ghost_cap),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some((val, freq)) = self.entries.get_mut(key) {
            *freq = std::cmp::min(3, *freq + 1);
            Some(val)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if let Some((val, freq)) = self.entries.get_mut(&key) {
            *val = value;
            *freq = std::cmp::min(3, *freq + 1);
            return;
        }

        while self.entries.len() >= self.capacity {
            self.evict();
        }

        // If in ghost queue, admit directly into Main queue
        if self.ghost_set.remove(&key).is_some() {
            self.entries.insert(key.clone(), (value, 0));
            self.main_queue.push_back(key);
        } else {
            self.entries.insert(key.clone(), (value, 0));
            self.small_queue.push_back(key);
        }
    }

    fn evict(&mut self) {
        // Evict from Small if it exceeds its quota, else from Main
        if self.small_queue.len() >= self.small_cap {
            while let Some(k) = self.small_queue.pop_front() {
                if let Some((_, freq)) = self.entries.get(&k) {
                    if *freq > 1 {
                        // Promoted to Main
                        self.main_queue.push_back(k);
                    } else {
                        // Evicted to Ghost
                        self.entries.remove(&k);
                        self.insert_ghost(k);
                        break;
                    }
                }
            }
        } else {
            while let Some(k) = self.main_queue.pop_front() {
                if let Some((_, freq)) = self.entries.get_mut(&k) {
                    if *freq > 0 {
                        *freq -= 1;
                        self.main_queue.push_back(k);
                    } else {
                        self.entries.remove(&k);
                        break;
                    }
                }
            }
        }
    }

    fn insert_ghost(&mut self, key: K) {
        if self.ghost_queue.len() >= self.ghost_cap {
            if let Some(old) = self.ghost_queue.pop_front() {
                self.ghost_set.remove(&old);
            }
        }
        self.ghost_set.insert(key.clone(), ());
        self.ghost_queue.push_back(key);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_fifo_cache() {
        let mut cache = S3FifoCache::new(5);
        cache.insert("a", 100);
        cache.insert("b", 200);
        cache.insert("c", 300);
        cache.insert("d", 400);
        cache.insert("e", 500);

        // Access "a" and "b" to increment frequency
        assert_eq!(cache.get(&"a"), Some(&100));
        assert_eq!(cache.get(&"b"), Some(&200));

        // Insert "f" -> should evict one of unaccessed items
        cache.insert("f", 600);
        assert_eq!(cache.len(), 5);
        assert_eq!(cache.get(&"f"), Some(&600));
    }
}
