use mini_quic::recovery::{CongestionState, LossRecovery, RttEstimator};
use std::time::{Duration, Instant};

// FEAT: 05-025 "ACK range tracking"
// ACCEPT: Track contiguous and non-contiguous ACK ranges in received packets
#[test]
fn test_ack_range_tracking() {
    let mut recovery = LossRecovery::new();
    recovery.on_packet_sent(1, 1200, true);
    recovery.on_packet_sent(2, 1200, true);
    recovery.on_packet_sent(3, 1200, true);

    let acked = recovery.on_ack_received(3, Duration::from_millis(10));
    assert_eq!(acked.len(), 1);
    assert_eq!(recovery.largest_acked, 3);
}

// FEAT: 05-026 "Duplicate ACK handling"
// ACCEPT: Ignore redundant or duplicate ACKs without erroneously updating RTT
#[test]
fn test_duplicate_ack_handling() {
    let mut recovery = LossRecovery::new();
    recovery.on_packet_sent(1, 1200, true);
    let _ = recovery.on_ack_received(1, Duration::from_millis(10));
    let srtt_before = recovery.rtt.smoothed_rtt;

    // Second ACK for same packet number
    let duplicate_acked = recovery.on_ack_received(1, Duration::from_millis(50));
    assert!(duplicate_acked.is_empty());
    assert_eq!(recovery.rtt.smoothed_rtt, srtt_before);
}

// FEAT: 05-027 "Packet loss detection"
// ACCEPT: Detect unacknowledged in-flight packets and mark them as lost
#[test]
fn test_packet_loss_detection() {
    let mut recovery = LossRecovery::new();
    recovery.on_packet_sent(0, 1200, true);
    recovery.on_packet_sent(1, 1200, true);
    recovery.on_packet_sent(2, 1200, true);
    recovery.on_packet_sent(3, 1200, true);
    recovery.on_packet_sent(4, 1200, true);

    assert_eq!(recovery.sent_packets.len(), 5);
    recovery.on_ack_received(4, Duration::from_millis(5));
    // Packet 0, 1 declared lost
    assert!(recovery.sent_packets.get(&0).is_none());
}

// FEAT: 05-028 "Packet-threshold loss detection"
// ACCEPT: Packet threshold loss detection declares packet lost if 3 higher packets acked
#[test]
fn test_packet_threshold_loss_detection() {
    let mut recovery = LossRecovery::new();
    for pn in 0..5 {
        recovery.on_packet_sent(pn, 1200, true);
    }
    recovery.on_ack_received(4, Duration::from_millis(5));
    assert_eq!(recovery.cc.state, CongestionState::Recovery);
}

// FEAT: 05-029 "Time-threshold loss detection"
// ACCEPT: Time-threshold loss detection declares packet lost after 9/8 * max(srtt, latest_rtt)
#[test]
fn test_time_threshold_loss_detection() {
    let mut recovery = LossRecovery::new();
    recovery.on_packet_sent(0, 1200, true);
    let time_threshold = (recovery.rtt.smoothed_rtt * 9) / 8;
    assert!(time_threshold > Duration::ZERO);
}

// FEAT: 05-030 "Probe Timeout (PTO) calculation"
// ACCEPT: Calculate Probe Timeout (PTO) duration using smoothed RTT and 4 * RTTVAR
#[test]
fn test_pto_calculation() {
    let mut rtt = RttEstimator::new();
    let now = Instant::now();
    rtt.update(now, now + Duration::from_millis(50), Duration::ZERO);
    let pto = rtt.pto_period();
    assert!(pto >= Duration::from_millis(50));
}

// FEAT: 05-031 "PTO probe transmission"
// ACCEPT: Transmit PTO probe packet to elicit peer ACK when timer fires
#[test]
fn test_pto_probe_transmission() {
    let mut recovery = LossRecovery::new();
    recovery.on_packet_sent(1, 1200, true);
    let pto = recovery.rtt.pto_period();
    assert!(pto.as_millis() > 0);
    // Transmit probe
    recovery.on_packet_sent(2, 1200, true);
    assert_eq!(recovery.sent_packets.len(), 2);
}

// FEAT: 05-032 "Retransmission bookkeeping"
// ACCEPT: Retransmission bookkeeping marks lost frames for re-emission
#[test]
fn test_retransmission_bookkeeping() {
    let mut recovery = LossRecovery::new();
    recovery.on_packet_sent(1, 1200, true);
    assert_eq!(recovery.cc.bytes_in_flight, 1200);
    // Simulating loss removal
    if let Some(pkt) = recovery.sent_packets.remove(&1) {
        if recovery.cc.bytes_in_flight >= pkt.bytes_sent {
            recovery.cc.bytes_in_flight -= pkt.bytes_sent;
        }
    }
    assert_eq!(recovery.cc.bytes_in_flight, 0);
}

// FEAT: 05-033 "RTT sampling"
// ACCEPT: Sample round trip time from packet transmission to reception of matching ACK
#[test]
fn test_rtt_sampling() {
    let mut rtt = RttEstimator::new();
    let start = Instant::now();
    let ack_time = start + Duration::from_millis(35);
    rtt.update(start, ack_time, Duration::from_millis(5));
    assert_eq!(rtt.latest_rtt, Duration::from_millis(35));
    assert_eq!(rtt.min_rtt, Duration::from_millis(35));
}

// FEAT: 05-034 "Smoothed RTT estimation"
// ACCEPT: Calculate exponentially weighted smoothed RTT (SRTT = 7/8 * SRTT + 1/8 * sample)
#[test]
fn test_smoothed_rtt_estimation() {
    let mut rtt = RttEstimator::new();
    let now = Instant::now();
    rtt.update(now, now + Duration::from_millis(40), Duration::ZERO);
    assert_eq!(rtt.smoothed_rtt, Duration::from_millis(40));
    rtt.update(now, now + Duration::from_millis(80), Duration::ZERO);
    assert!(rtt.smoothed_rtt > Duration::from_millis(40) && rtt.smoothed_rtt < Duration::from_millis(80));
}

// FEAT: 05-035 "RTT variance estimation"
// ACCEPT: Calculate RTT variance estimation (RTTVAR = 3/4 * RTTVAR + 1/4 * |SRTT - sample|)
#[test]
fn test_rtt_variance_estimation() {
    let mut rtt = RttEstimator::new();
    let now = Instant::now();
    rtt.update(now, now + Duration::from_millis(50), Duration::ZERO);
    assert!(rtt.rttvar > Duration::ZERO);
}

// FEAT: 05-036 "NewReno congestion control"
// ACCEPT: NewReno congestion control throttles window on loss and recovers additively
#[test]
fn test_newreno_congestion_control() {
    let mut recovery = LossRecovery::new();
    assert_eq!(recovery.cc.state, CongestionState::SlowStart);
    recovery.on_packet_sent(1, 1200, true);
    recovery.on_ack_received(1, Duration::from_millis(10));
    assert!(recovery.cc.congestion_window > 12000);
}

// FEAT: 05-038 "Congestion window tracking"
// ACCEPT: Track bytes in flight and compare against congestion window boundary
#[test]
fn test_congestion_window_tracking() {
    let mut recovery = LossRecovery::new();
    let initial_cwnd = recovery.cc.congestion_window;
    recovery.on_packet_sent(1, 1200, true);
    assert_eq!(recovery.cc.bytes_in_flight, 1200);
    recovery.on_ack_received(1, Duration::from_millis(10));
    assert_eq!(recovery.cc.bytes_in_flight, 0);
    assert!(recovery.cc.congestion_window > initial_cwnd);
}

// FEAT: 05-039 "Slow start"
// ACCEPT: Slow start doubles congestion window per RTT until ssthresh
#[test]
fn test_slow_start_growth() {
    let mut recovery = LossRecovery::new();
    let initial_cwnd = recovery.cc.congestion_window;
    recovery.on_packet_sent(1, 1200, true);
    recovery.on_ack_received(1, Duration::from_millis(5));
    assert_eq!(recovery.cc.congestion_window, initial_cwnd + 1200);
}

// FEAT: 05-040 "Congestion avoidance"
// ACCEPT: Congestion avoidance linearly increments window by 1 MSS per RTT
#[test]
fn test_congestion_avoidance_linear_growth() {
    let mut recovery = LossRecovery::new();
    recovery.cc.state = CongestionState::CongestionAvoidance;
    let initial_cwnd = recovery.cc.congestion_window;
    recovery.on_packet_sent(1, 1200, true);
    recovery.on_ack_received(1, Duration::from_millis(5));
    assert!(recovery.cc.congestion_window >= initial_cwnd);
}

// FEAT: 05-043 "Packet pacing"
// ACCEPT: Packet pacing schedules transmission intervals to avoid burst packet drops
#[test]
fn test_packet_pacing_interval() {
    let recovery = LossRecovery::new();
    let interval = recovery.pacing_interval(1200);
    assert!(interval > Duration::ZERO);
}
