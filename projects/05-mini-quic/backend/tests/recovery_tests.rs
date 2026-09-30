use mini_quic::recovery::{CongestionState, LossRecovery};
use std::time::Duration;

#[test]
fn test_rtt_estimator_and_congestion_avoidance() {
    let mut recovery = LossRecovery::new();

    // Send 10 packets
    for pn in 0..10 {
        recovery.on_packet_sent(pn, 1200, true);
    }

    assert_eq!(recovery.cc.state, CongestionState::SlowStart);
    assert_eq!(recovery.cc.bytes_in_flight, 12000);

    // Ack packet 9 with 50ms delay
    let acked = recovery.on_ack_received(9, Duration::from_millis(5));
    assert_eq!(acked.len(), 1);
    assert!(recovery.rtt.smoothed_rtt > Duration::ZERO);

    // Trigger loss via 3-packet gap
    recovery.on_packet_sent(10, 1200, true);
    recovery.on_packet_sent(11, 1200, true);
    recovery.on_packet_sent(12, 1200, true);
    recovery.on_packet_sent(13, 1200, true);

    recovery.on_ack_received(13, Duration::from_millis(5));
    assert_eq!(recovery.cc.state, CongestionState::Recovery);
}
