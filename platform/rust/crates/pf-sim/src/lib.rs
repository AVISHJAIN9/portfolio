use rand::Rng;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Deterministic Pseudo-Random Number Generator.
pub struct DeterministicRng {
    rng: Xoshiro256PlusPlus,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Xoshiro256PlusPlus::seed_from_u64(seed),
        }
    }

    pub fn gen_range<T: rand::distributions::uniform::SampleUniform + PartialOrd>(
        &mut self,
        range: std::ops::Range<T>,
    ) -> T {
        self.rng.gen_range(range)
    }

    pub fn gen_bool(&mut self, p: f64) -> bool {
        self.rng.gen_bool(p)
    }
}

/// Scheduled Simulation Event.
#[derive(Debug)]
struct ScheduledEvent<E> {
    timestamp_ms: u64,
    event_id: u64,
    payload: E,
}

impl<E> PartialEq for ScheduledEvent<E> {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp_ms == other.timestamp_ms && self.event_id == other.event_id
    }
}

impl<E> Eq for ScheduledEvent<E> {}

impl<E> PartialOrd for ScheduledEvent<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E> Ord for ScheduledEvent<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap (earliest timestamp first)
        other
            .timestamp_ms
            .cmp(&self.timestamp_ms)
            .then_with(|| other.event_id.cmp(&self.event_id))
    }
}

/// Deterministic Discrete-Event Simulator (DES).
pub struct DiscreteEventSimulator<E> {
    current_time_ms: u64,
    event_counter: u64,
    queue: BinaryHeap<ScheduledEvent<E>>,
}

impl<E> Default for DiscreteEventSimulator<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> DiscreteEventSimulator<E> {
    pub fn new() -> Self {
        Self {
            current_time_ms: 0,
            event_counter: 0,
            queue: BinaryHeap::new(),
        }
    }

    pub fn schedule_after(&mut self, delay_ms: u64, payload: E) {
        self.event_counter += 1;
        self.queue.push(ScheduledEvent {
            timestamp_ms: self.current_time_ms + delay_ms,
            event_id: self.event_counter,
            payload,
        });
    }

    pub fn step(&mut self) -> Option<(u64, E)> {
        let event = self.queue.pop()?;
        self.current_time_ms = event.timestamp_ms;
        Some((event.timestamp_ms, event.payload))
    }

    pub fn now(&self) -> u64 {
        self.current_time_ms
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discrete_event_sim() {
        let mut sim = DiscreteEventSimulator::new();
        sim.schedule_after(100, "event_b");
        sim.schedule_after(20, "event_a");
        sim.schedule_after(150, "event_c");

        let (t1, e1) = sim.step().unwrap();
        assert_eq!((t1, e1), (20, "event_a"));

        let (t2, e2) = sim.step().unwrap();
        assert_eq!((t2, e2), (100, "event_b"));

        let (t3, e3) = sim.step().unwrap();
        assert_eq!((t3, e3), (150, "event_c"));
    }
}
