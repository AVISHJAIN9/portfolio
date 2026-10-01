//! In-Memory MemTable with Concurrent Skiplist & Size Threshold Tracking (06-027..035)

use crossbeam_skiplist::SkipMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueEntry {
    Value(Vec<u8>),
    Tombstone,
}

pub struct MemTable {
    map: Arc<SkipMap<Vec<u8>, ValueEntry>>,
    approx_size_bytes: AtomicUsize,
}

impl Default for MemTable {
    fn default() -> Self {
        Self::new()
    }
}

impl MemTable {
    pub fn new() -> Self {
        Self {
            map: Arc::new(SkipMap::new()),
            approx_size_bytes: AtomicUsize::new(0),
        }
    }

    pub fn put(&self, key: &[u8], value: &[u8]) {
        let entry_size = key.len() + value.len() + 16;
        self.map.insert(key.to_vec(), ValueEntry::Value(value.to_vec()));
        self.approx_size_bytes.fetch_add(entry_size, Ordering::Relaxed);
    }

    pub fn delete(&self, key: &[u8]) {
        let entry_size = key.len() + 16;
        self.map.insert(key.to_vec(), ValueEntry::Tombstone);
        self.approx_size_bytes.fetch_add(entry_size, Ordering::Relaxed);
    }

    pub fn get(&self, key: &[u8]) -> Option<ValueEntry> {
        self.map.get(key).map(|entry| entry.value().clone())
    }

    pub fn size_bytes(&self) -> usize {
        self.approx_size_bytes.load(Ordering::Relaxed)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> Vec<(Vec<u8>, ValueEntry)> {
        self.map
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}
