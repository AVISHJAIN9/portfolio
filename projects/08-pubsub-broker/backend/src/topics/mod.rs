//! Topics and Partition Routing (Features 08-014..08-026)

use crc32fast::Hasher;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionMetadata {
    pub partition_id: u32,
    pub leader_node_id: u64,
    pub replicas: Vec<u64>,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMetadata {
    pub name: String,
    pub partition_count: u32,
    pub replication_factor: u32,
    pub retention_ms: u64,
    pub partitions: Vec<PartitionMetadata>,
}

/// 08-017..08-026: Topic Registry and Partition Router
pub struct TopicRegistry {
    topics: RwLock<HashMap<String, TopicMetadata>>,
    round_robin_counters: RwLock<HashMap<String, Arc<AtomicU32>>>,
    partition_message_counts: RwLock<HashMap<(String, u32), Arc<AtomicU64>>>,
}

impl Default for TopicRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TopicRegistry {
    pub fn new() -> Self {
        Self {
            topics: RwLock::new(HashMap::new()),
            round_robin_counters: RwLock::new(HashMap::new()),
            partition_message_counts: RwLock::new(HashMap::new()),
        }
    }

    /// 08-014..08-016: Create topic with partitions
    pub fn create_topic(&self, name: String, partition_count: u32, replication_factor: u32, retention_ms: u64) -> Result<(), &'static str> {
        if partition_count == 0 {
            return Err("Partition count must be > 0");
        }
        let mut topics = self.topics.write();
        if topics.contains_key(&name) {
            return Err("Topic already exists");
        }

        let mut partitions = Vec::with_capacity(partition_count as usize);
        for p in 0..partition_count {
            partitions.push(PartitionMetadata {
                partition_id: p,
                leader_node_id: (p % 3) as u64 + 1, // Demo leader node
                replicas: vec![1, 2, 3],
                is_available: true,
            });
            self.partition_message_counts.write().insert((name.clone(), p), Arc::new(AtomicU64::new(0)));
        }

        topics.insert(
            name.clone(),
            TopicMetadata {
                name: name.clone(),
                partition_count,
                replication_factor,
                retention_ms,
                partitions,
            },
        );
        self.round_robin_counters.write().insert(name, Arc::new(AtomicU32::new(0)));
        Ok(())
    }

    /// 08-006: Delete topic
    pub fn delete_topic(&self, name: &str) -> bool {
        self.topics.write().remove(name).is_some()
    }

    /// 08-017..08-018: Route message to partition by key or round-robin
    pub fn route_partition(&self, topic: &str, key: Option<&[u8]>) -> Result<u32, &'static str> {
        let topics = self.topics.read();
        let meta = topics.get(topic).ok_or("Topic not found")?;
        let count = meta.partition_count;

        if let Some(k) = key {
            // 08-017: Deterministic hash routing
            let mut hasher = Hasher::new();
            hasher.update(k);
            let hash = hasher.finalize();
            Ok(hash % count)
        } else {
            // 08-018: Round-robin routing
            let rr_map = self.round_robin_counters.read();
            if let Some(counter) = rr_map.get(topic) {
                let next = counter.fetch_add(1, Ordering::Relaxed);
                Ok(next % count)
            } else {
                Ok(0)
            }
        }
    }

    /// Record message ingestion for metrics and hot-partition tracking
    pub fn record_partition_message(&self, topic: &str, partition: u32) {
        let map = self.partition_message_counts.read();
        if let Some(c) = map.get(&(topic.to_string(), partition)) {
            c.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 08-024: Detect hot partition (skew ratio > threshold)
    pub fn is_hot_partition(&self, topic: &str, partition: u32, threshold_ratio: f64) -> bool {
        let topics = self.topics.read();
        let meta = match topics.get(topic) {
            Some(m) => m,
            None => return false,
        };
        let map = self.partition_message_counts.read();
        let mut total = 0u64;
        let mut target_count = 0u64;

        for p in 0..meta.partition_count {
            if let Some(c) = map.get(&(topic.to_string(), p)) {
                let count = c.load(Ordering::Relaxed);
                total += count;
                if p == partition {
                    target_count = count;
                }
            }
        }

        if total == 0 {
            return false;
        }
        let avg = total as f64 / meta.partition_count as f64;
        (target_count as f64 / avg) > threshold_ratio
    }

    pub fn get_topic(&self, name: &str) -> Option<TopicMetadata> {
        self.topics.read().get(name).cloned()
    }
}
