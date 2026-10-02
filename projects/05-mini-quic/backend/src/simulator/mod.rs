//! Deterministic Network Simulator with Virtual Clock & Impairment Injection (05-064..05-068)

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SimulatedPacket {
    pub id: u64,
    pub payload: Vec<u8>,
    pub deliver_at_ns: u64,
}

impl PartialEq for SimulatedPacket {
    fn eq(&self, other: &Self) -> bool {
        self.deliver_at_ns == other.deliver_at_ns && self.id == other.id
    }
}

impl Eq for SimulatedPacket {}

impl PartialOrd for SimulatedPacket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimulatedPacket {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap by delivery time
        other
            .deliver_at_ns
            .cmp(&self.deliver_at_ns)
            .then_with(|| other.id.cmp(&self.id))
    }
}

pub struct NetworkSimulator {
    rng: StdRng,
    virtual_time_ns: u64,
    queue: BinaryHeap<SimulatedPacket>,
    next_packet_id: u64,
    pub latency_mean_ms: f64,
    pub latency_jitter_ms: f64,
    pub loss_rate: f64,
    pub duplication_rate: f64,
    pub reorder_rate: f64,
    pub packets_sent: u64,
    pub packets_dropped: u64,
    pub packets_duplicated: u64,
    pub packets_delivered: u64,
}

impl NetworkSimulator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            virtual_time_ns: 0,
            queue: BinaryHeap::new(),
            next_packet_id: 1,
            latency_mean_ms: 10.0,
            latency_jitter_ms: 2.0,
            loss_rate: 0.0,
            duplication_rate: 0.0,
            reorder_rate: 0.0,
            packets_sent: 0,
            packets_dropped: 0,
            packets_duplicated: 0,
            packets_delivered: 0,
        }
    }

    pub fn send(&mut self, payload: Vec<u8>) {
        self.packets_sent += 1;

        // 1. Loss injection (05-066)
        if self.loss_rate > 0.0 && self.rng.gen::<f64>() < self.loss_rate {
            self.packets_dropped += 1;
            return;
        }

        // 2. Latency calculation (05-065)
        let jitter = if self.latency_jitter_ms > 0.0 {
            self.rng
                .gen_range(-self.latency_jitter_ms..=self.latency_jitter_ms)
        } else {
            0.0
        };
        let delay_ms = (self.latency_mean_ms + jitter).max(0.1);
        let delay_ns = (delay_ms * 1_000_000.0) as u64;

        // 3. Reordering injection (05-068)
        let effective_delay_ns =
            if self.reorder_rate > 0.0 && self.rng.gen::<f64>() < self.reorder_rate {
                delay_ns / 2 // Deliver earlier to create out-of-order delivery
            } else {
                delay_ns
            };

        let deliver_at_ns = self.virtual_time_ns + effective_delay_ns;
        let pkt_id = self.next_packet_id;
        self.next_packet_id += 1;

        self.queue.push(SimulatedPacket {
            id: pkt_id,
            payload: payload.clone(),
            deliver_at_ns,
        });

        // 4. Duplication injection (05-067)
        if self.duplication_rate > 0.0 && self.rng.gen::<f64>() < self.duplication_rate {
            self.packets_duplicated += 1;
            self.queue.push(SimulatedPacket {
                id: self.next_packet_id,
                payload,
                deliver_at_ns: deliver_at_ns + 500_000, // +0.5ms duplicate
            });
            self.next_packet_id += 1;
        }
    }

    pub fn step(&mut self, advance_duration: Duration) -> Vec<SimulatedPacket> {
        self.virtual_time_ns += advance_duration.as_nanos() as u64;
        let mut ready = Vec::new();
        while let Some(top) = self.queue.peek() {
            if top.deliver_at_ns <= self.virtual_time_ns {
                let pkt = self.queue.pop().unwrap();
                self.packets_delivered += 1;
                ready.push(pkt);
            } else {
                break;
            }
        }
        ready
    }

    pub fn current_virtual_time_ms(&self) -> f64 {
        self.virtual_time_ns as f64 / 1_000_000.0
    }
}
