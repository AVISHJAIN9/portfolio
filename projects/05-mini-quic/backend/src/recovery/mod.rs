//! QUIC Loss Detection and Congestion Control (RFC 9002)

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

pub const INITIAL_RTT: Duration = Duration::from_millis(100);
pub const MIN_RTT_WINDOW: Duration = Duration::from_secs(10);
pub const MAX_PACKET_SIZE: u64 = 1200;
pub const INITIAL_WINDOW_PACKETS: u64 = 10;
pub const MIN_CONGESTION_WINDOW: u64 = 2 * MAX_PACKET_SIZE;

#[derive(Debug, Clone)]
pub struct SentPacket {
    pub packet_number: u64,
    pub time_sent: Instant,
    pub bytes_sent: u64,
    pub is_ack_eliciting: bool,
    pub in_flight: bool,
}

pub struct RttEstimator {
    pub latest_rtt: Duration,
    pub smoothed_rtt: Duration,
    pub rttvar: Duration,
    pub min_rtt: Duration,
    pub min_rtt_timestamp: Instant,
}

impl Default for RttEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl RttEstimator {
    pub fn new() -> Self {
        Self {
            latest_rtt: Duration::ZERO,
            smoothed_rtt: INITIAL_RTT,
            rttvar: INITIAL_RTT / 2,
            min_rtt: Duration::MAX,
            min_rtt_timestamp: Instant::now(),
        }
    }

    pub fn update(&mut self, send_time: Instant, ack_time: Instant, ack_delay: Duration) {
        if ack_time < send_time {
            return;
        }
        let sample = ack_time - send_time;
        self.latest_rtt = sample;

        if self.min_rtt == Duration::MAX || sample < self.min_rtt {
            self.min_rtt = sample;
            self.min_rtt_timestamp = ack_time;
        }

        let adjusted_rtt = if sample >= self.min_rtt + ack_delay {
            sample - ack_delay
        } else {
            sample
        };

        if self.smoothed_rtt == INITIAL_RTT && self.rttvar == INITIAL_RTT / 2 {
            self.smoothed_rtt = sample;
            self.rttvar = sample / 2;
        } else {
            let diff = self.smoothed_rtt.abs_diff(adjusted_rtt);
            self.rttvar = (self.rttvar * 3 + diff) / 4;
            self.smoothed_rtt = (self.smoothed_rtt * 7 + adjusted_rtt) / 8;
        }
    }

    pub fn pto_period(&self) -> Duration {
        self.smoothed_rtt + std::cmp::max(4 * self.rttvar, Duration::from_millis(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionState {
    SlowStart,
    CongestionAvoidance,
    Recovery,
}

pub struct CongestionController {
    pub state: CongestionState,
    pub congestion_window: u64,
    pub bytes_in_flight: u64,
    pub ssthresh: u64,
    pub recovery_start_time: Option<Instant>,
}

impl Default for CongestionController {
    fn default() -> Self {
        Self::new()
    }
}

impl CongestionController {
    pub fn new() -> Self {
        Self {
            state: CongestionState::SlowStart,
            congestion_window: INITIAL_WINDOW_PACKETS * MAX_PACKET_SIZE,
            bytes_in_flight: 0,
            ssthresh: u64::MAX,
            recovery_start_time: None,
        }
    }

    pub fn on_packet_sent(&mut self, bytes: u64) {
        self.bytes_in_flight += bytes;
    }

    pub fn on_packet_acked(&mut self, bytes: u64) {
        if self.bytes_in_flight >= bytes {
            self.bytes_in_flight -= bytes;
        } else {
            self.bytes_in_flight = 0;
        }

        match self.state {
            CongestionState::SlowStart => {
                self.congestion_window += bytes;
                if self.congestion_window >= self.ssthresh {
                    self.state = CongestionState::CongestionAvoidance;
                }
            }
            CongestionState::CongestionAvoidance => {
                // Additive increase: 1 MSS per RTT
                let increment = (MAX_PACKET_SIZE * MAX_PACKET_SIZE) / self.congestion_window;
                self.congestion_window += std::cmp::max(1, increment);
            }
            CongestionState::Recovery => {
                // Maintained until recovery period exits
            }
        }
    }

    pub fn on_congestion_event(&mut self, now: Instant) {
        if let Some(start) = self.recovery_start_time {
            if now < start {
                return;
            }
        }
        self.recovery_start_time = Some(now);
        self.ssthresh = std::cmp::max(self.congestion_window / 2, MIN_CONGESTION_WINDOW);
        self.congestion_window = self.ssthresh;
        self.state = CongestionState::Recovery;
    }

    pub fn can_send(&self) -> bool {
        self.bytes_in_flight < self.congestion_window
    }
}

pub struct LossRecovery {
    pub rtt: RttEstimator,
    pub cc: CongestionController,
    pub sent_packets: BTreeMap<u64, SentPacket>,
    pub largest_acked: u64,
    pub loss_time: Option<Instant>,
}

impl Default for LossRecovery {
    fn default() -> Self {
        Self::new()
    }
}

impl LossRecovery {
    pub fn new() -> Self {
        Self {
            rtt: RttEstimator::new(),
            cc: CongestionController::new(),
            sent_packets: BTreeMap::new(),
            largest_acked: 0,
            loss_time: None,
        }
    }

    pub fn on_packet_sent(&mut self, pn: u64, bytes: u64, is_ack_eliciting: bool) {
        self.sent_packets.insert(pn, SentPacket {
            packet_number: pn,
            time_sent: Instant::now(),
            bytes_sent: bytes,
            is_ack_eliciting,
            in_flight: true,
        });
        self.cc.on_packet_sent(bytes);
    }

    pub fn on_ack_received(&mut self, largest_acked: u64, ack_delay: Duration) -> Vec<SentPacket> {
        let now = Instant::now();
        let mut acked_packets = Vec::new();
        self.largest_acked = std::cmp::max(self.largest_acked, largest_acked);

        if let Some(pkt) = self.sent_packets.remove(&largest_acked) {
            self.rtt.update(pkt.time_sent, now, ack_delay);
            self.cc.on_packet_acked(pkt.bytes_sent);
            acked_packets.push(pkt);
        }

        // Detect lost packets by packet threshold (3 packets gap) or time threshold
        let loss_delay = (self.rtt.smoothed_rtt * 9) / 8;
        let mut lost_pns = Vec::new();

        for (&pn, pkt) in &self.sent_packets {
            if pn < self.largest_acked
                && (self.largest_acked - pn >= 3 || now.duration_since(pkt.time_sent) >= loss_delay) {
                lost_pns.push(pn);
            }
        }

        if !lost_pns.is_empty() {
            self.cc.on_congestion_event(now);
            for pn in lost_pns {
                if let Some(lost) = self.sent_packets.remove(&pn) {
                    if self.cc.bytes_in_flight >= lost.bytes_sent {
                        self.cc.bytes_in_flight -= lost.bytes_sent;
                    }
                }
            }
        }

        acked_packets
    }
}
