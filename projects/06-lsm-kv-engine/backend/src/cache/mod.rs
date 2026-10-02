//! Block Cache with Sharding, LRU Eviction, Admission Policy, and Hit/Miss Metrics (06-092..06-096)

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const NUM_SHARDS: usize = 16;

#[derive(Debug, Clone)]
struct CacheEntry {
    value: Arc<Vec<u8>>,
    size: usize,
    last_access: u64,
    access_count: u32,
}

struct CacheShard {
    entries: HashMap<Vec<u8>, CacheEntry>,
    current_bytes: usize,
    capacity_bytes: usize,
}

impl CacheShard {
    fn new(capacity_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            current_bytes: 0,
            capacity_bytes,
        }
    }

    fn evict_lru(&mut self, needed_bytes: usize) {
        while self.current_bytes + needed_bytes > self.capacity_bytes && !self.entries.is_empty() {
            let oldest_key = self
                .entries
                .iter()
                .min_by_key(|(_, v)| v.last_access)
                .map(|(k, _)| k.clone());

            if let Some(key) = oldest_key {
                if let Some(removed) = self.entries.remove(&key) {
                    self.current_bytes = self.current_bytes.saturating_sub(removed.size);
                }
            } else {
                break;
            }
        }
    }
}

pub struct BlockCache {
    shards: Vec<RwLock<CacheShard>>,
    total_capacity: usize,
    access_clock: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl BlockCache {
    pub fn new(capacity_bytes: usize) -> Self {
        let shard_cap = capacity_bytes.max(NUM_SHARDS) / NUM_SHARDS;
        let mut shards = Vec::with_capacity(NUM_SHARDS);
        for _ in 0..NUM_SHARDS {
            shards.push(RwLock::new(CacheShard::new(shard_cap)));
        }
        Self {
            shards,
            total_capacity: capacity_bytes,
            access_clock: AtomicU64::new(1),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    fn shard_index(&self, key: &[u8]) -> usize {
        let mut h: usize = 5381;
        for &b in key {
            h = ((h << 5).wrapping_add(h)).wrapping_add(b as usize);
        }
        h % NUM_SHARDS
    }

    pub fn get(&self, key: &[u8]) -> Option<Arc<Vec<u8>>> {
        let shard_idx = self.shard_index(key);
        let mut shard = self.shards[shard_idx].write();
        if let Some(entry) = shard.entries.get_mut(key) {
            let clock = self.access_clock.fetch_add(1, Ordering::Relaxed);
            entry.last_access = clock;
            entry.access_count = entry.access_count.saturating_add(1);
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(Arc::clone(&entry.value))
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn insert(&self, key: &[u8], value: Vec<u8>) -> Result<(), &'static str> {
        self.insert_with_admission(key, value, false)
    }

    pub fn insert_with_admission(
        &self,
        key: &[u8],
        value: Vec<u8>,
        is_scan: bool,
    ) -> Result<(), &'static str> {
        // Admission policy: large scans or compaction reads bypass the cache
        if is_scan {
            return Ok(());
        }

        let entry_size = key.len() + value.len() + 32;
        let shard_idx = self.shard_index(key);
        let mut shard = self.shards[shard_idx].write();

        if entry_size > shard.capacity_bytes {
            return Err("entry size exceeds shard capacity limit");
        }

        shard.evict_lru(entry_size);

        let clock = self.access_clock.fetch_add(1, Ordering::Relaxed);
        let entry = CacheEntry {
            value: Arc::new(value),
            size: entry_size,
            last_access: clock,
            access_count: 1,
        };

        if let Some(old) = shard.entries.insert(key.to_vec(), entry) {
            shard.current_bytes = shard.current_bytes.saturating_sub(old.size);
        }
        shard.current_bytes += entry_size;
        Ok(())
    }

    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    pub fn total_bytes_used(&self) -> usize {
        self.shards.iter().map(|s| s.read().current_bytes).sum()
    }

    pub fn capacity(&self) -> usize {
        self.total_capacity
    }

    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    pub fn hit_ratio(&self) -> f64 {
        let h = self.hits() as f64;
        let m = self.misses() as f64;
        if h + m == 0.0 {
            0.0
        } else {
            h / (h + m)
        }
    }
}
