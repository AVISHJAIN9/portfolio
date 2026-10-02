//! State, TTL & Deduplication (Features 07-053..07-065)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateRecord {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
    pub ttl_seconds: u64,
}

/// 07-053..07-057: Keyed State Store with TTL
pub struct KeyedStateStore {
    store: HashMap<String, StateRecord>,
}

impl KeyedStateStore {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn put(&mut self, key: impl Into<String>, value: impl Into<String>, ttl_s: u64) {
        let key = key.into();
        self.store.insert(
            key.clone(),
            StateRecord {
                key,
                value: value.into(),
                updated_at: Utc::now(),
                ttl_seconds: ttl_s,
            },
        );
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        let record = self.store.get(key)?;
        let elapsed = Utc::now()
            .signed_duration_since(record.updated_at)
            .num_seconds();
        if record.ttl_seconds > 0 && elapsed > record.ttl_seconds as i64 {
            None
        } else {
            Some(&record.value)
        }
    }

    pub fn sweep_ttl(&mut self) -> usize {
        let now = Utc::now();
        let before = self.store.len();
        self.store.retain(|_, record| {
            if record.ttl_seconds == 0 {
                true
            } else {
                let elapsed = now.signed_duration_since(record.updated_at).num_seconds();
                elapsed <= record.ttl_seconds as i64
            }
        });
        before - self.store.len()
    }

    /// 07-060: Snapshot & Checksum serialization
    pub fn snapshot(&self) -> (Vec<u8>, String) {
        let data = serde_json::to_vec(&self.store).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum = format!("{:x}", hasher.finalize());
        (data, checksum)
    }

    pub fn restore(&mut self, data: &[u8], checksum: &str) -> Result<(), &'static str> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual = format!("{:x}", hasher.finalize());
        if actual != checksum {
            return Err("Checksum mismatch on state restore");
        }
        self.store = serde_json::from_slice(data).map_err(|_| "Deserialization error")?;
        Ok(())
    }
}

impl Default for KeyedStateStore {
    fn default() -> Self {
        Self::new()
    }
}

/// 07-063: Event & Alert Deduplication Engine
pub struct Deduplicator {
    seen_keys: HashMap<String, DateTime<Utc>>,
    ttl_seconds: i64,
}

impl Deduplicator {
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            seen_keys: HashMap::new(),
            ttl_seconds,
        }
    }

    /// Returns true if key is fresh (not duplicate), false if already seen
    pub fn check_and_insert(&mut self, key: &str, now: DateTime<Utc>) -> bool {
        if let Some(&ts) = self.seen_keys.get(key) {
            if (now - ts).num_seconds() <= self.ttl_seconds {
                return false; // Duplicate
            }
        }
        self.seen_keys.insert(key.to_string(), now);
        true
    }
}
