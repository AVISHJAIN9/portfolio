use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HealthStatus {
    Ok,
    Degraded(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineHealthReport {
    pub status: HealthStatus,
    pub active_levels: usize,
    pub memtable_bytes: usize,
    pub pending_compaction_bytes: u64,
    pub wal_size_bytes: u64,
    pub is_stalled: bool,
}

#[derive(Debug, Default)]
pub struct EngineMetrics {
    pub put_count: AtomicU64,
    pub get_count: AtomicU64,
    pub delete_count: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub total_latency_nanos: AtomicU64,
}

impl EngineMetrics {
    pub fn record_op(&self, latency_nanos: u64) {
        self.total_latency_nanos.fetch_add(latency_nanos, Ordering::Relaxed);
    }

    pub fn snapshot_counters(&self) -> (u64, u64, u64, u64, u64) {
        (
            self.put_count.load(Ordering::Relaxed),
            self.get_count.load(Ordering::Relaxed),
            self.delete_count.load(Ordering::Relaxed),
            self.cache_hits.load(Ordering::Relaxed),
            self.cache_misses.load(Ordering::Relaxed),
        )
    }
}

/// Limits & Validation Configuration (06-118, 06-119)
#[derive(Debug, Clone)]
pub struct EngineLimits {
    pub max_key_size_bytes: usize,
    pub max_value_size_bytes: usize,
    pub allow_empty_keys: bool,
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            max_key_size_bytes: 64 * 1024,      // 64 KB max key
            max_value_size_bytes: 16 * 1024 * 1024, // 16 MB max value
            allow_empty_keys: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError {
    EmptyKey,
    KeyTooLarge { actual: usize, max: usize },
    ValueTooLarge { actual: usize, max: usize },
}

impl EngineLimits {
    pub fn validate_put(&self, key: &[u8], value: &[u8]) -> Result<(), ValidationError> {
        if key.is_empty() && !self.allow_empty_keys {
            return Err(ValidationError::EmptyKey);
        }
        if key.len() > self.max_key_size_bytes {
            return Err(ValidationError::KeyTooLarge {
                actual: key.len(),
                max: self.max_key_size_bytes,
            });
        }
        if value.len() > self.max_value_size_bytes {
            return Err(ValidationError::ValueTooLarge {
                actual: value.len(),
                max: self.max_value_size_bytes,
            });
        }
        Ok(())
    }

    pub fn validate_key(&self, key: &[u8]) -> Result<(), ValidationError> {
        if key.is_empty() && !self.allow_empty_keys {
            return Err(ValidationError::EmptyKey);
        }
        if key.len() > self.max_key_size_bytes {
            return Err(ValidationError::KeyTooLarge {
                actual: key.len(),
                max: self.max_key_size_bytes,
            });
        }
        Ok(())
    }
}
