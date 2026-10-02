//! Windows & Event Time (Features 07-040..07-052)

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// 07-040..07-043: Window Types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WindowType {
    Tumbling(u64), // size in seconds
    Sliding { size_s: u64, slide_s: u64 },
    Session(u64), // gap in seconds
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowKey {
    pub rule_id: String,
    pub key: String,
    pub window_start: i64,
    pub window_end: i64,
}

/// 07-044..07-047: Watermark Manager for Out-of-Order Handling
pub struct WatermarkManager {
    partition_watermarks: HashMap<u32, DateTime<Utc>>,
    allowed_lateness: Duration,
}

impl WatermarkManager {
    pub fn new(allowed_lateness: Duration) -> Self {
        Self {
            partition_watermarks: HashMap::new(),
            allowed_lateness,
        }
    }

    /// Updates watermark for a partition and returns the global minimum watermark
    pub fn update_watermark(&mut self, partition: u32, event_time: DateTime<Utc>) -> DateTime<Utc> {
        let wm = event_time - self.allowed_lateness;
        self.partition_watermarks
            .entry(partition)
            .and_modify(|e| {
                if wm > *e {
                    *e = wm;
                }
            })
            .or_insert(wm);

        *self.partition_watermarks.values().min().unwrap_or(&wm)
    }

    pub fn is_late(&self, partition: u32, event_time: DateTime<Utc>) -> bool {
        if let Some(&wm) = self.partition_watermarks.get(&partition) {
            event_time < wm
        } else {
            false
        }
    }
}

/// 07-040: Window Accumulator Store
pub struct WindowAccumulator {
    windows: HashMap<WindowKey, Vec<String>>, // Window -> Event IDs
}

impl WindowAccumulator {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    pub fn assign_tumbling(
        &mut self,
        rule_id: &str,
        key: &str,
        event_time: DateTime<Utc>,
        event_id: &str,
        window_size_s: u64,
    ) -> WindowKey {
        let ts = event_time.timestamp();
        let start = (ts / window_size_s as i64) * window_size_s as i64;
        let end = start + window_size_s as i64;

        let w_key = WindowKey {
            rule_id: rule_id.to_string(),
            key: key.to_string(),
            window_start: start,
            window_end: end,
        };

        self.windows
            .entry(w_key.clone())
            .or_default()
            .push(event_id.to_string());
        w_key
    }

    pub fn get_count(&self, w_key: &WindowKey) -> usize {
        self.windows.get(w_key).map(|v| v.len()).unwrap_or(0)
    }

    pub fn purge_expired(&mut self, min_watermark_ts: i64) -> usize {
        let before_len = self.windows.len();
        self.windows.retain(|k, _| k.window_end > min_watermark_ts);
        before_len - self.windows.len()
    }
}

impl Default for WindowAccumulator {
    fn default() -> Self {
        Self::new()
    }
}
