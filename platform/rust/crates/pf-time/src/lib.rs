use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

/// Hybrid Logical Clock timestamp (Physical millis, Logical counter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HlcTimestamp {
    pub physical_ms: u64,
    pub logical: u32,
}

impl PartialOrd for HlcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HlcTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.physical_ms
            .cmp(&other.physical_ms)
            .then_with(|| self.logical.cmp(&other.logical))
    }
}

/// Hybrid Logical Clock Generator (Kulkarni et al., 2014).
pub struct HybridLogicalClock {
    latest: parking_lot::Mutex<HlcTimestamp>,
    pub max_drift_ms: u64,
}

impl HybridLogicalClock {
    pub fn new(max_drift_ms: u64) -> Self {
        Self {
            latest: parking_lot::Mutex::new(HlcTimestamp {
                physical_ms: 0,
                logical: 0,
            }),
            max_drift_ms,
        }
    }

    /// Generate next timestamp for local event.
    pub fn now(&self) -> HlcTimestamp {
        let physical = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut lock = self.latest.lock();
        if physical > lock.physical_ms {
            lock.physical_ms = physical;
            lock.logical = 0;
        } else {
            lock.logical += 1;
        }
        *lock
    }

    /// Update clock upon receiving a remote message timestamp.
    pub fn update(&self, remote: HlcTimestamp) -> HlcTimestamp {
        let physical = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut lock = self.latest.lock();
        let max_physical = std::cmp::max(
            physical,
            std::cmp::max(lock.physical_ms, remote.physical_ms),
        );

        if max_physical == lock.physical_ms && max_physical == remote.physical_ms {
            lock.logical = std::cmp::max(lock.logical, remote.logical) + 1;
        } else if max_physical == lock.physical_ms {
            lock.logical += 1;
        } else if max_physical == remote.physical_ms {
            lock.physical_ms = remote.physical_ms;
            lock.logical = remote.logical + 1;
        } else {
            lock.physical_ms = max_physical;
            lock.logical = 0;
        }
        *lock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hlc_monotonicity() {
        let hlc = HybridLogicalClock::new(5000);
        let t1 = hlc.now();
        let t2 = hlc.now();
        assert!(t2 > t1);

        let remote = HlcTimestamp {
            physical_ms: t2.physical_ms + 100,
            logical: 5,
        };
        let t3 = hlc.update(remote);
        assert!(t3 > remote);
    }
}
