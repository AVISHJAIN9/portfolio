//! S3-FIFO Query Result Cache for OLAP Workloads

use pf_cache::S3FifoCache;
use std::sync::Mutex;

pub struct QueryResultCache {
    cache: Mutex<S3FifoCache<String, String>>,
}

impl QueryResultCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(S3FifoCache::new(capacity)),
        }
    }

    pub fn get(&self, query_sql: &str) -> Option<String> {
        let mut guard = self.cache.lock().unwrap();
        guard.get(&query_sql.to_string()).cloned()
    }

    pub fn put(&self, query_sql: &str, result_json: String) {
        let mut guard = self.cache.lock().unwrap();
        guard.insert(query_sql.to_string(), result_json);
    }
}
