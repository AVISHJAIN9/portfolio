use hdrhistogram::Histogram;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// HDR Latency Histogram tracking microsecond-precision distributions.
pub struct LatencyHistogram {
    hist: Mutex<Histogram<u64>>,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyHistogram {
    pub fn new() -> Self {
        Self {
            hist: Mutex::new(Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap()),
        }
    }

    pub fn record(&self, micros: u64) {
        let mut lock = self.hist.lock();
        let _ = lock.record(std::cmp::max(1, micros));
    }

    pub fn p50(&self) -> u64 {
        self.hist.lock().value_at_quantile(0.50)
    }

    pub fn p95(&self) -> u64 {
        self.hist.lock().value_at_quantile(0.95)
    }

    pub fn p99(&self) -> u64 {
        self.hist.lock().value_at_quantile(0.99)
    }
}

/// Atomic Monotonic Counter.
pub struct Counter {
    val: AtomicU64,
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl Counter {
    pub const fn new() -> Self {
        Self {
            val: AtomicU64::new(0),
        }
    }

    pub fn inc(&self) -> u64 {
        self.val.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn add(&self, delta: u64) -> u64 {
        self.val.fetch_add(delta, Ordering::Relaxed) + delta
    }

    pub fn get(&self) -> u64 {
        self.val.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_and_counter() {
        let hist = LatencyHistogram::new();
        hist.record(100);
        hist.record(200);
        hist.record(1000);
        assert!(hist.p50() >= 100);
        assert!(hist.p99() >= 200);

        let counter = Counter::new();
        counter.inc();
        counter.add(5);
        assert_eq!(counter.get(), 6);
    }
}
