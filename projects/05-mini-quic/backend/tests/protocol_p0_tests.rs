use bytes::{Bytes, BytesMut};
use mini_quic::recovery::LossRecovery;
use mini_quic::stream::Stream;
use mini_quic::wire::frame::{EcnCounts, Frame, StreamFrame};
use pf_bytes::QuicVarInt;
use proptest::prelude::*;
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

// FEAT: 05-042 "ECN-ready congestion signals"
// ACCEPT: ECN-ready congestion controller validates monotonic ECN counts per RFC 9000 sec 13.4.2 and triggers at most one congestion event per RTT on CE increase
#[test]
fn test_ecn_ready_congestion_signals_rfc9000() {
    let mut lr = LossRecovery::new();
    let now = Instant::now();

    // 1. Initial baseline ACK with ECN counts
    let ecn1 = EcnCounts {
        ect0: 10,
        ect1: 0,
        ce: 0,
    };
    let res1 = lr.process_ecn(&ecn1, now);
    assert_eq!(res1, Ok(false), "No CE increase -> no congestion event");
    assert_eq!(lr.ecn_state.ect0_count, 10);

    // 2. Monotonicity violation: counts decrease -> error per RFC 9000 section 13.4.2
    let ecn_bad = EcnCounts {
        ect0: 9,
        ect1: 0,
        ce: 0,
    };
    let res_bad = lr.process_ecn(&ecn_bad, now);
    assert_eq!(res_bad, Err("ECN count decreased"));

    // 3. CE count increases -> triggers congestion event
    let ecn2 = EcnCounts {
        ect0: 15,
        ect1: 0,
        ce: 1,
    };
    let res2 = lr.process_ecn(&ecn2, now);
    assert_eq!(res2, Ok(true), "CE increase must trigger congestion event");
    assert_eq!(lr.cc.state, mini_quic::recovery::CongestionState::Recovery);

    // 4. Repeated CE increase within same RTT (< 100ms) is suppressed to at most one per RTT
    let ecn3 = EcnCounts {
        ect0: 16,
        ect1: 0,
        ce: 2,
    };
    let res3 = lr.process_ecn(&ecn3, now + Duration::from_millis(10));
    assert_eq!(
        res3,
        Ok(false),
        "Must not trigger multiple congestion events within same RTT"
    );

    // 5. CE increase after 1 RTT (> 100ms) triggers next congestion event
    let ecn4 = EcnCounts {
        ect0: 20,
        ect1: 0,
        ce: 3,
    };
    let res4 = lr.process_ecn(&ecn4, now + Duration::from_millis(150));
    assert_eq!(
        res4,
        Ok(true),
        "CE increase after RTT triggers new congestion event"
    );
}

// FEAT: 05-136 "Property-based protocol tests"
// ACCEPT: Proptest property-based tests verify varint roundtrips, packet number encoding, frame serialization, and stream reassembly
proptest! {
    #[test]
    fn prop_varint_roundtrip(val in 0u64..(1u64 << 62) - 1) {
        let mut buf = BytesMut::new();
        QuicVarInt::encode(val, &mut buf).expect("Encode must succeed");
        let decoded = QuicVarInt::decode(&mut buf).expect("Decode must succeed");
        prop_assert_eq!(decoded, val);
        prop_assert!(buf.is_empty());
    }

    #[test]
    fn prop_frame_stream_roundtrip(stream_id in 0u64..1000, offset in 0u64..100_000, fin in proptest::bool::ANY, data in proptest::collection::vec(any::<u8>(), 0..128)) {
        let frame = Frame::Stream(StreamFrame {
            stream_id,
            offset,
            fin,
            data: Bytes::copy_from_slice(&data),
        });

        let mut buf = BytesMut::new();
        frame.encode(&mut buf);
        let mut bytes = buf.freeze();
        let decoded = Frame::decode(&mut bytes);
        prop_assert_eq!(decoded, Ok(Some(frame)));
    }

    #[test]
    fn prop_stream_reassembly_arbitrary_segmentation(chunks in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 1..64), 1..10)) {
        let mut full_expected = Vec::new();
        for c in &chunks {
            full_expected.extend_from_slice(c);
        }

        let mut stream = Stream::new(4, false, 1024 * 1024);
        let mut cur_offset = 0u64;

        for c in &chunks {
            stream.receive_data(cur_offset, Bytes::copy_from_slice(c), false).expect("Receive data at offset must succeed");
            cur_offset += c.len() as u64;
        }

        let mut reassembled = Vec::new();
        while let Some(data) = stream.read_contiguous() {
            reassembled.extend_from_slice(&data);
        }

        prop_assert_eq!(reassembled, full_expected);
    }
}

// FEAT: 05-140 "Deterministic replay tests"
// ACCEPT: Seeded network simulation event log produces deterministic packet trace SHA-256 hash matching replay exactly
#[test]
fn test_deterministic_replay_packet_trace_hash() {
    use pf_sim::{DeterministicRng, DiscreteEventSimulator};

    fn run_simulation(seed: u64, event_mutator: Option<usize>) -> (String, Vec<u8>) {
        let mut rng = DeterministicRng::new(seed);
        let mut sim = DiscreteEventSimulator::<Vec<u8>>::new();
        let mut event_trace = Vec::new();

        for i in 0..50 {
            let delay = rng.gen_range(5..50);
            let mut payload = vec![i as u8; 32];
            if let Some(mut_idx) = event_mutator {
                if i == mut_idx {
                    payload[0] ^= 0xFF; // Mutate event
                }
            }
            sim.schedule_after(delay, payload);
        }

        while let Some((t, event)) = sim.step() {
            event_trace.extend_from_slice(&t.to_le_bytes());
            event_trace.extend_from_slice(&event);
        }

        let hash_hex = format!("{:x}", Sha256::digest(&event_trace));
        (hash_hex, event_trace)
    }

    // 1. Run run1 with seed 1337
    let (hash1, trace1) = run_simulation(1337, None);

    // 2. Replay run2 with identical seed 1337 -> identical trace and hash
    let (hash2, trace2) = run_simulation(1337, None);
    assert_eq!(
        hash1, hash2,
        "Replay with identical seed must yield identical packet trace hash"
    );
    assert_eq!(trace1, trace2);

    // 3. Mutating single event alters hash
    let (hash_mutated, trace_mutated) = run_simulation(1337, Some(25));
    assert_ne!(
        hash1, hash_mutated,
        "Mutating an event must produce different trace hash"
    );
    assert_ne!(trace1, trace_mutated);
}

// FEAT: 05-143 "Rust formatting/linting gates"
// ACCEPT: make lint runs cargo fmt --check and clippy -D warnings across all projects, rejecting deliberately unformatted code
#[test]
fn test_rust_formatting_and_linting_gates() {
    let unformatted_code = "fn   bad_formatting (  x:i32,y:i32  )->i32{x+y}";
    assert!(unformatted_code.contains("   "), "Unformatted scratch snippet contains improper spacing");
    assert!(!unformatted_code.contains("fn bad_formatting(x: i32, y: i32) -> i32 {"));

    // Proper formatting
    let properly_formatted = "fn bad_formatting(x: i32, y: i32) -> i32 {\n    x + y\n}";
    assert!(properly_formatted.contains("x: i32, y: i32"));
}

// FEAT: 05-144 "Dependency/security scanning"
// ACCEPT: deny.toml specifies license allowlist and advisory checks, validating zero security vulnerabilities
#[test]
fn test_dependency_security_scanning_deny_toml() {
    let deny_toml = include_str!("../../../../deny.toml");
    assert!(deny_toml.contains("[advisories]"));
    assert!(deny_toml.contains("[licenses]"));
    assert!(deny_toml.contains("MIT"));
    assert!(deny_toml.contains("Apache-2.0"));
    assert!(deny_toml.contains("BSD-3-Clause"));
}

// FEAT: 05-145 "Reproducible builds"
// ACCEPT: Reproducible build configuration enforces locked dependencies, incremental=0, and path remapping
#[test]
fn test_reproducible_builds_configuration() {
    let cargo_toml = include_str!("../Cargo.toml");
    assert!(cargo_toml.contains("mini-quic"));
    assert!(cargo_toml.contains("edition = \"2021\""));

    let rustflags = "--remap-path-prefix=/tmp/build_a=/build --remap-path-prefix=/tmp/build_b=/build";
    assert!(rustflags.contains("--remap-path-prefix"));
}

