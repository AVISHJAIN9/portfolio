//! Log Retention & Key Compaction (Features 08-105..08-117)

use crate::model::Record;
use std::collections::HashMap;

/// 08-105..08-117: Log Compactor & Retention Evaluator
pub struct Compactor;

impl Compactor {
    /// 08-108..08-112: Compact records by retaining the latest record per key and handling tombstones (empty value)
    pub fn compact_records(records: Vec<Record>) -> Vec<Record> {
        let mut latest_by_key: HashMap<Vec<u8>, Record> = HashMap::new();
        let mut unkeyed_records = Vec::new();

        for record in records {
            if let Some(ref key) = record.key {
                if record.value.is_empty() {
                    // Tombstone: delete previous versions
                    latest_by_key.remove(key);
                } else {
                    latest_by_key.insert(key.clone(), record);
                }
            } else {
                unkeyed_records.push(record);
            }
        }

        let mut compacted: Vec<Record> = latest_by_key.into_values().collect();
        compacted.extend(unkeyed_records);
        compacted.sort_by_key(|r| r.offset);
        compacted
    }

    /// 08-105: Time-based retention filter
    pub fn filter_expired(records: Vec<Record>, retention_cutoff_ts: i64) -> Vec<Record> {
        records
            .into_iter()
            .filter(|r| r.timestamp >= retention_cutoff_ts)
            .collect()
    }
}
