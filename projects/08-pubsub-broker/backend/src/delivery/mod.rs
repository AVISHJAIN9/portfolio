//! At-Least-Once Delivery & Dead Letter Queue (Features 08-092..08-104)

use crate::model::Record;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflightMessage {
    pub record: Record,
    pub consumer_id: String,
    pub dispatched_at: u64,
    pub redelivery_count: u32,
}

/// 08-092..08-104: Delivery Coordinator & DLQ Manager
pub struct DeliveryCoordinator {
    inflight: Mutex<HashMap<(u32, u64), InflightMessage>>, // (partition, offset) -> Inflight
    dlq: Mutex<Vec<Record>>,
    max_redelivery_attempts: u32,
    ack_timeout_secs: u64,
}

impl DeliveryCoordinator {
    pub fn new(max_redelivery_attempts: u32, ack_timeout_secs: u64) -> Self {
        Self {
            inflight: Mutex::new(HashMap::new()),
            dlq: Mutex::new(Vec::new()),
            max_redelivery_attempts,
            ack_timeout_secs,
        }
    }

    /// 08-092: Dispatch message to consumer
    pub fn dispatch(&self, partition: u32, record: Record, consumer_id: &str, now: u64) {
        let mut inflight = self.inflight.lock();
        let key = (partition, record.offset);
        let count = inflight.get(&key).map_or(0, |m| m.redelivery_count);
        inflight.insert(
            key,
            InflightMessage {
                record,
                consumer_id: consumer_id.to_string(),
                dispatched_at: now,
                redelivery_count: count,
            },
        );
    }

    /// 08-010: Acknowledge message delivery
    pub fn acknowledge(&self, partition: u32, offset: u64) -> bool {
        let mut inflight = self.inflight.lock();
        inflight.remove(&(partition, offset)).is_some()
    }

    /// 08-095..08-098: Check for timed-out messages, trigger redelivery or route to DLQ
    pub fn process_timeouts(&self, now: u64) -> Vec<(u32, Record)> {
        let mut inflight = self.inflight.lock();
        let mut dlq = self.dlq.lock();
        let mut redeliver_list = Vec::new();

        let mut to_remove = Vec::new();

        for (&(partition, offset), msg) in inflight.iter_mut() {
            if now >= msg.dispatched_at + self.ack_timeout_secs {
                msg.redelivery_count += 1;
                if msg.redelivery_count >= self.max_redelivery_attempts {
                    // 08-098: Route poison/unacked message to Dead Letter Queue
                    let mut dead_record = msg.record.clone();
                    dead_record.headers.insert("x-dead-letter-reason".to_string(), "max_redelivery_exceeded".to_string());
                    dlq.push(dead_record);
                    to_remove.push((partition, offset));
                } else {
                    // Reschedule for redelivery
                    msg.dispatched_at = now;
                    redeliver_list.push((partition, msg.record.clone()));
                }
            }
        }

        for k in to_remove {
            inflight.remove(&k);
        }

        redeliver_list
    }

    pub fn dlq_len(&self) -> usize {
        self.dlq.lock().len()
    }

    pub fn inflight_len(&self) -> usize {
        self.inflight.lock().len()
    }
}
