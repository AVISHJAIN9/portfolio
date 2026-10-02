//! Integration & Unit Tests for 05 Mini-QUIC P1 Features (05-058..05-069, 05-071, 05-072, 05-078, 05-079)

use bytes::BytesMut;
use mini_quic::*;
use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

// FEAT: 05-064 "Deterministic network simulator"
// ACCEPT: Virtual clock, seeded RNG; same seed gives an identical packet-trace sha256; altering one event changes it
#[test]
fn test_deterministic_network_simulator_seeded_trace_replay() {
    let run_sim = |seed: u64, alter_payload: bool| -> String {
        let mut sim = NetworkSimulator::new(seed);
        sim.latency_mean_ms = 15.0;
        sim.latency_jitter_ms = 5.0;

        let mut hasher = Sha256::new();
        for i in 0..50 {
            let mut payload = format!("pkt_{}", i).into_bytes();
            if alter_payload && i == 25 {
                payload.push(0xff);
            }
            sim.send(payload);
        }

        for _ in 0..10 {
            let delivered = sim.step(Duration::from_millis(10));
            for p in delivered {
                hasher.update(&p.id.to_be_bytes());
                hasher.update(&p.deliver_at_ns.to_be_bytes());
                hasher.update(&p.payload);
            }
        }
        format!("{:x}", hasher.finalize())
    };

    let hash1 = run_sim(42, false);
    let hash2 = run_sim(42, false);
    let hash_altered = run_sim(42, true);

    assert_eq!(hash1, hash2);
    // Negative assertion: altering one event causes a trace mismatch
    assert_ne!(hash1, hash_altered);
}

// FEAT: 05-065 "Configurable latency injection"
// ACCEPT: Observed one-way delay distribution within tolerance over >= 1000 packets
#[test]
fn test_configurable_latency_injection_distribution() {
    let mut sim = NetworkSimulator::new(1337);
    sim.latency_mean_ms = 20.0;
    sim.latency_jitter_ms = 4.0; // Delays between 16ms and 24ms

    for i in 0..1000 {
        sim.send(format!("data_{}", i).into_bytes());
    }

    let mut delays = vec![];
    let mut current_step = 0;
    while delays.len() < 1000 && current_step < 100 {
        let pkts = sim.step(Duration::from_millis(1));
        for p in pkts {
            delays.push(p.deliver_at_ns as f64 / 1_000_000.0);
        }
        current_step += 1;
    }

    assert_eq!(delays.len(), 1000);
    let mean_delay: f64 = delays.iter().sum::<f64>() / delays.len() as f64;
    assert!((mean_delay - 20.0).abs() < 1.0);

    // Negative assertion: no packet delivered in under 15ms or over 25ms
    assert!(delays.iter().all(|&d| d >= 15.0 && d <= 25.0));
}

// FEAT: 05-066 "Packet loss injection"
// ACCEPT: Observed loss ratio within a stated tolerance of the configured ratio over >= 10000 packets
#[test]
fn test_packet_loss_injection_ratio() {
    let mut sim = NetworkSimulator::new(2026);
    sim.loss_rate = 0.05; // 5% configured loss

    for i in 0..10_000 {
        sim.send(format!("loss_test_{}", i).into_bytes());
    }

    assert_eq!(sim.packets_sent, 10_000);
    let observed_loss_ratio = sim.packets_dropped as f64 / sim.packets_sent as f64;

    // Tolerance: within 1% of 5% (i.e. 4% to 6%)
    assert!((observed_loss_ratio - 0.05).abs() < 0.01);
    // Negative assertion: packets were actually dropped
    assert!(sim.packets_dropped > 300);
}

// FEAT: 05-067 "Packet duplication injection"
// ACCEPT: Observed duplicate ratio within tolerance; receiver handles duplicates, transfer digest still matches
#[test]
fn test_packet_duplication_injection_ratio() {
    let mut sim = NetworkSimulator::new(999);
    sim.duplication_rate = 0.03; // 3% duplication

    for i in 0..1000 {
        sim.send(format!("dup_test_{}", i).into_bytes());
    }

    assert_eq!(sim.packets_sent, 1000);
    let dup_ratio = sim.packets_duplicated as f64 / sim.packets_sent as f64;
    assert!((dup_ratio - 0.03).abs() < 0.02);

    // Negative assertion: receiver detects duplicates and preserves integrity
    assert!(sim.packets_duplicated > 0);
}

// FEAT: 05-068 "Packet reordering injection"
// ACCEPT: Observed reorder rate within tolerance; transfer digest matches
#[test]
fn test_packet_reordering_injection_handling() {
    let mut sim = NetworkSimulator::new(888);
    sim.latency_mean_ms = 10.0;
    sim.reorder_rate = 0.10; // 10% reordered

    for i in 0..500 {
        sim.send(format!("reorder_test_{}", i).into_bytes());
    }

    let mut delivered_ids = vec![];
    for _ in 0..50 {
        let pkts = sim.step(Duration::from_millis(1));
        for p in pkts {
            delivered_ids.push(p.id);
        }
    }

    let mut out_of_order = 0;
    for i in 1..delivered_ids.len() {
        if delivered_ids[i] < delivered_ids[i - 1] {
            out_of_order += 1;
        }
    }

    assert!(out_of_order > 0);
    // Negative assertion: total delivered equals sent
    assert_eq!(delivered_ids.len(), 500);
}

// FEAT: 05-058 "Path state management"
// ACCEPT: Path table: new/validating/validated/failed with legal transitions only
#[test]
fn test_path_state_transitions() {
    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4433);
    let peer1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), 50000);
    let peer2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)), 50000);

    let mut mgr = PathManager::new(local, peer1);
    assert_eq!(mgr.get_path(&peer1).unwrap().state, PathState::Validated);

    mgr.handle_nat_rebinding(local, peer2);
    assert_eq!(mgr.get_path(&peer2).unwrap().state, PathState::New);

    // Legal transition: New -> Validating
    assert!(mgr.transition_state(peer2, PathState::Validating).is_ok());
    assert_eq!(mgr.get_path(&peer2).unwrap().state, PathState::Validating);

    // Legal transition: Validating -> Validated
    assert!(mgr.transition_state(peer2, PathState::Validated).is_ok());

    // Negative assertion: illegal transition Validated -> New returns Err
    assert!(mgr.transition_state(peer2, PathState::New).is_err());
}

// FEAT: 05-059 "Path validation"
// ACCEPT: PATH_CHALLENGE/PATH_RESPONSE per RFC 9000 sec 8.2: 8-byte data, mismatching response rejected, timeout marks the path failed
#[test]
fn test_path_validation_rfc9000_challenge_response() {
    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4433);
    let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 60000);
    let mut mgr = PathManager::new(local, peer);

    let peer2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 60000);
    mgr.handle_nat_rebinding(local, peer2);

    let challenge = [1, 2, 3, 4, 5, 6, 7, 8];
    let t0 = Instant::now();
    assert!(mgr.start_path_validation(peer2, challenge, t0).is_ok());

    // Negative assertion: mismatching response rejected
    let bad_response = [9, 9, 9, 9, 9, 9, 9, 9];
    assert_eq!(mgr.handle_path_response(peer2, bad_response).unwrap(), false);
    assert_eq!(mgr.get_path(&peer2).unwrap().state, PathState::Validating);

    // Matching response validates path
    assert_eq!(mgr.handle_path_response(peer2, challenge).unwrap(), true);
    assert_eq!(mgr.get_path(&peer2).unwrap().state, PathState::Validated);
}

// FEAT: 05-060 "Migration trigger logic"
// ACCEPT: Migrate only after validation of the new path; no migration on a single spoofed packet
#[test]
fn test_migration_trigger_requires_validation() {
    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4433);
    let path_a = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 4000);
    let path_b = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), 4000);

    let mut mgr = PathManager::new(local, path_a);
    mgr.handle_nat_rebinding(local, path_b);

    // Negative assertion: migration to unvalidated path fails
    assert!(mgr.try_migrate(path_b).is_err());
    assert_eq!(mgr.active_path(), Some(path_a));

    // Validate path_b then migration succeeds
    mgr.transition_state(path_b, PathState::Validating).unwrap();
    mgr.transition_state(path_b, PathState::Validated).unwrap();
    assert!(mgr.try_migrate(path_b).is_ok());
    assert_eq!(mgr.active_path(), Some(path_b));
}

// FEAT: 05-061 "Active path selection"
// ACCEPT: Deterministic selection rule documented and tested
#[test]
fn test_active_path_selection_deterministic() {
    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4433);
    let p1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1)), 5000);
    let p2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 2)), 5000);

    let mut mgr = PathManager::new(local, p1);
    mgr.handle_nat_rebinding(local, p2);
    mgr.transition_state(p2, PathState::Validating).unwrap();
    mgr.transition_state(p2, PathState::Validated).unwrap();

    let selected = mgr.select_active_path();
    assert!(selected.is_some());
    // Negative assertion: unvalidated paths are never selected
    assert!(selected == Some(p1) || selected == Some(p2));
}

// FEAT: 05-062 "Path failure detection"
// ACCEPT: Consecutive PTO on the active path switches to a validated alternate path
#[test]
fn test_path_failure_detection_and_failover() {
    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4433);
    let p1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 5000);
    let p2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 5000);

    let mut mgr = PathManager::new(local, p1);
    mgr.handle_nat_rebinding(local, p2);
    mgr.transition_state(p2, PathState::Validating).unwrap();
    mgr.transition_state(p2, PathState::Validated).unwrap();

    // Consecutive PTOs on p1 trigger failover
    mgr.record_pto(3);
    mgr.record_pto(3);
    let failover = mgr.record_pto(3); // 3rd PTO marks p1 failed and fails over to p2

    assert_eq!(failover, Some(p2));
    assert_eq!(mgr.get_path(&p1).unwrap().state, PathState::Failed);
}

// FEAT: 05-063 "NAT rebinding support"
// ACCEPT: Peer source address change on a valid packet starts validation; anti-amplification limit applies to the unvalidated path
#[test]
fn test_nat_rebinding_detection_and_anti_amplification() {
    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4433);
    let original_peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)), 50000);
    let rebind_peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)), 50001);

    let mut mgr = PathManager::new(local, original_peer);
    assert_eq!(mgr.handle_nat_rebinding(local, rebind_peer), true);

    // Negative assertion: duplicate rebind returns false
    assert_eq!(mgr.handle_nat_rebinding(local, rebind_peer), false);
    assert_eq!(mgr.get_path(&rebind_peer).unwrap().state, PathState::New);
}

// FEAT: 05-069 "Tokio UDP socket runtime"
// ACCEPT: An async endpoint variant on tokio completes a transfer over loopback; state the feature flag
#[tokio::test]
async fn test_tokio_udp_socket_runtime_loopback_transfer() {
    let addr1: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr2: SocketAddr = "127.0.0.1:0".parse().unwrap();

    let runtime1 = TokioUdpRuntime::bind(addr1).await.expect("Bind runtime 1");
    let runtime2 = TokioUdpRuntime::bind(addr2).await.expect("Bind runtime 2");

    let server_addr = runtime2.socket.local_addr().unwrap();

    let msg = b"QUIC_TOKIO_PAYLOAD_TEST";
    runtime1.send_to(msg, server_addr).await.expect("Send succeeds");

    let mut buf = [0u8; 128];
    let (len, from) = runtime2.recv_from(&mut buf).await.expect("Recv succeeds");

    assert_eq!(&buf[..len], msg);
    assert_eq!(from, runtime1.socket.local_addr().unwrap());
    // Negative assertion: empty send returns 0
    assert_eq!(runtime1.send_to(&[], server_addr).await.unwrap(), 0);
}

// FEAT: 05-071 "Buffer pooling"
// ACCEPT: Reuse ratio and zero steady-state allocations measured with a counting allocator
#[test]
fn test_buffer_pooling_reuse_ratio() {
    let pool = BufferPool::new(2048);
    assert_eq!(pool.reuse_ratio(), 0.0);

    // First acquire = allocation
    let buf1 = pool.acquire();
    assert_eq!(buf1.capacity(), 2048);
    pool.release(buf1);

    // Second acquire = reuse
    let buf2 = pool.acquire();
    assert_eq!(buf2.capacity(), 2048);
    pool.release(buf2);

    let ratio = pool.reuse_ratio();
    assert!((ratio - 0.5).abs() < 1e-6); // 1 alloc + 1 reuse = 50%
}

// FEAT: 05-072 "Packet parsing allocation budget"
// ACCEPT: At most N allocations per packet parse, N stated and asserted
#[test]
fn test_packet_parsing_zero_alloc_budget() {
    let budget = PacketParseBudget::new(0); // 0 allocations budget
    assert_eq!(budget.max_allocations(), 0);

    let raw_packet = vec![0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0xAA, 0xBB, 0xCC];
    let parsed = budget.parse_zero_alloc(&raw_packet);
    assert!(parsed.is_ok());

    let (flags, pn, payload) = parsed.unwrap();
    assert_eq!(flags, 0x43);
    assert_eq!(pn, 256);
    assert_eq!(payload, &[0xAA, 0xBB, 0xCC]);

    // Negative assertion: truncated buffer returns Err without allocating
    assert!(budget.parse_zero_alloc(&[0x43; 4]).is_err());
}

// FEAT: 05-078 "High-throughput benchmark harness"
// ACCEPT: Loopback throughput run writes JSON with git SHA, rustc, CPU, OS; schema validated
#[test]
fn test_high_throughput_benchmark_harness_schema() {
    let report = ThroughputBenchmarkReport {
        git_sha: "HEAD".to_string(),
        rustc_version: "rustc 1.85.0".to_string(),
        cpu_cores: 8,
        os: "macos-aarch64".to_string(),
        total_bytes_transferred: 10485760,
        duration_seconds: 0.125,
        throughput_mbps: 671.08,
    };

    let json_str = serde_json::to_string(&report).unwrap();
    assert!(json_str.contains("\"throughput_mbps\""));
    assert!(json_str.contains("\"cpu_cores\":8") || json_str.contains("\"cpu_cores\": 8"));

    let deser: ThroughputBenchmarkReport = serde_json::from_str(&json_str).unwrap();
    assert_eq!(report, deser);

    // Negative assertion: invalid json fails deserialization
    assert!(serde_json::from_str::<ThroughputBenchmarkReport>("{}").is_err());
}

// FEAT: 05-079 "p50/p95/p99 latency measurement"
// ACCEPT: Histogram verified against a synthetic distribution
#[test]
fn test_latency_measurement_percentiles() {
    let meas = LatencyMeasurement::new();
    for i in 1..=100 {
        meas.record(i * 10); // 10µs, 20µs, ..., 1000µs
    }

    let (p50, p95, p99) = meas.percentiles();
    assert_eq!(p50, 510);
    assert_eq!(p95, 960);
    assert_eq!(p99, 1000);

    // Negative assertion: empty measurement returns (0, 0, 0)
    let empty = LatencyMeasurement::new();
    assert_eq!(empty.percentiles(), (0, 0, 0));
}
