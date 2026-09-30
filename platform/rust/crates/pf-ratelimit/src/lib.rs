use parking_lot::Mutex;
use std::time::{Duration, Instant};

/// Generic Cell Rate Algorithm (GCRA) Rate Limiter (ITU-T I.371 / Leaky Bucket).
pub struct GcraLimiter {
    emission_interval: Duration,
    tolerance: Duration,
    tat: Mutex<Option<Instant>>,
}

impl GcraLimiter {
    pub fn new(rate_per_sec: u32, burst: u32) -> Self {
        let emission_interval = Duration::from_secs_f64(1.0 / rate_per_sec as f64);
        let tolerance = emission_interval * burst;
        Self {
            emission_interval,
            tolerance,
            tat: Mutex::new(None),
        }
    }

    /// Check if a cell/request is conforming. If conforming, updates TAT and returns Ok(()).
    pub fn check(&self) -> bool {
        self.check_at(Instant::now())
    }

    pub fn check_at(&self, now: Instant) -> bool {
        let mut tat_lock = self.tat.lock();
        let tat = match *tat_lock {
            Some(t) => t,
            None => {
                *tat_lock = Some(now + self.emission_interval);
                return true;
            }
        };

        if now + self.tolerance < tat {
            // Non-conforming: rate limit exceeded
            false
        } else {
            // Conforming: advance Theoretical Arrival Time
            let new_tat = std::cmp::max(now, tat) + self.emission_interval;
            *tat_lock = Some(new_tat);
            true
        }
    }
}

/// Token Bucket Rate Limiter with atomic replenishment.
pub struct TokenBucket {
    capacity: f64,
    refill_rate_per_sec: f64,
    tokens: Mutex<(f64, Instant)>,
}

impl TokenBucket {
    pub fn new(capacity: f64, refill_rate_per_sec: f64) -> Self {
        Self {
            capacity,
            refill_rate_per_sec,
            tokens: Mutex::new((capacity, Instant::now())),
        }
    }

    pub fn try_acquire(&self, count: f64) -> bool {
        let mut lock = self.tokens.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(lock.1).as_secs_f64();

        lock.0 = (lock.0 + elapsed * self.refill_rate_per_sec).min(self.capacity);
        lock.1 = now;

        if lock.0 >= count {
            lock.0 -= count;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcra_limiter() {
        let limiter = GcraLimiter::new(10, 2); // 10 req/s, burst of 2 (allows 1 + 2 = 3 immediate)
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(limiter.check());
        // 4th immediate request exceeds tolerance
        assert!(!limiter.check());
    }

    #[test]
    fn test_token_bucket() {
        let bucket = TokenBucket::new(5.0, 1.0);
        assert!(bucket.try_acquire(3.0));
        assert!(bucket.try_acquire(2.0));
        assert!(!bucket.try_acquire(1.0));
    }
}
