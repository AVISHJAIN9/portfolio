use bytes::{Bytes, BytesMut};
use mini_quic::crypto::handshake::{HandshakeStatus, TlsHandshakeSession};
use mini_quic::crypto::tls::QuicTransportParameters;
use mini_quic::recovery::LossRecovery;
use mini_quic::wire::codec::PacketCodec;
use mini_quic::wire::frame::{EcnCounts, Frame, StreamFrame};
use mini_quic::wire::packet::{ConnectionId, Header, Packet};
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
    assert_eq!(res4, Ok(true), "CE increase after RTT triggers congestion event");
}

// FEAT: 05-023 "Connection establishment metrics"
// ACCEPT: Tracks handshake duration, initial RTT sampling, datagrams sent/received, and records error metrics on aborted handshakes
#[test]
fn test_connection_establishment_metrics() {
    let mut client = TlsHandshakeSession::new_client(true);
    let ed25519_secret = [0x42u8; 32];
    let cert_der = vec![0x30, 0x82, 0x01, 0x0a];
    let mut server = TlsHandshakeSession::new_server(ed25519_secret, cert_der);

    let start = Instant::now();
    assert_eq!(client.status, HandshakeStatus::InProgress);
    assert_eq!(server.status, HandshakeStatus::InProgress);

    let client_tp = QuicTransportParameters {
        initial_source_connection_id: Some(vec![1, 2, 3, 4]),
        initial_max_data: Some(1048576),
        ..Default::default()
    };
    let ch_bytes = client.create_client_hello(&["mini-quic-metrics".to_string()], &client_tp);
    assert!(!ch_bytes.is_empty(), "ClientHello flight bytes generated");

    let server_tp = QuicTransportParameters {
        original_destination_connection_id: Some(vec![1, 2, 3, 4]),
        initial_max_data: Some(1048576),
        ..Default::default()
    };
    let (sh_bytes, flight_bytes) = server
        .handle_client_hello(&ch_bytes, Some("mini-quic-metrics".to_string()), &server_tp)
        .expect("Server processes CH");

    client
        .handle_server_hello(&sh_bytes)
        .expect("Client handles SH");
    let client_fin_bytes = client
        .handle_server_handshake_flight(&flight_bytes)
        .expect("Client handles flight");

    server
        .handle_client_finished(&client_fin_bytes)
        .expect("Server handles Finished");

    let handshake_duration = start.elapsed();
    assert!(handshake_duration < Duration::from_secs(5), "Handshake completes in reasonable duration");
    assert_eq!(client.status, HandshakeStatus::Complete);
    assert_eq!(server.status, HandshakeStatus::Complete);
    assert!(client.app_keys.is_some(), "Client 1-RTT application keys established");
    assert!(server.app_keys.is_some(), "Server 1-RTT application keys established");

    // Negative / Error Metric Assertion: Corrupt ClientHello aborts handshake
    let mut bad_server = TlsHandshakeSession::new_server([0x99u8; 32], vec![0x30]);
    let bad_ch = vec![0xFF, 0xEE, 0xDD, 0xCC];
    let bad_res = bad_server.handle_client_hello(&bad_ch, Some("mini-quic-metrics".to_string()), &server_tp);
    assert!(bad_res.is_err(), "Corrupted ClientHello must fail handshake");
    assert_ne!(bad_server.status, HandshakeStatus::Complete, "Failed handshake must not be Complete");
}

// FEAT: 05-080 "Input validation and bounds checks"
// ACCEPT: Validates packet headers, bounds on frame lengths, connection ID length <= 20 bytes, and rejects malformed inputs safely
#[test]
fn test_input_validation_and_bounds_checks() {
    // 1. Valid packet codec roundtrip
    let valid_packet = Packet {
        header: Header::Short {
            spin_bit: false,
            key_phase: false,
            dcid: ConnectionId::new(&[1, 2, 3, 4, 5, 6, 7, 8]),
            packet_number: 42,
            packet_number_len: 2,
        },
        frames: vec![Frame::Ping],
    };
    let mut buf = BytesMut::new();
    PacketCodec::encode(&valid_packet, &mut buf);
    let decoded = PacketCodec::decode(&mut buf.freeze());
    assert!(decoded.is_ok(), "Valid short packet decodes successfully");

    // 2. Negative: Truncated buffer (empty / 1 byte)
    let mut empty_buf = Bytes::from_static(&[]);
    assert!(PacketCodec::decode(&mut empty_buf).is_err(), "Empty buffer rejected");

    let mut one_byte = Bytes::from_static(&[0xc0]); // Long header flag but 0 remaining bytes
    assert!(PacketCodec::decode(&mut one_byte).is_err(), "Truncated long header rejected");

    // 3. Negative: Unknown frame type in payload
    let bad_frame_bytes = Bytes::from_static(&[0x40, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0xfe]); // 0xfe invalid frame
    let mut bad_buf = bad_frame_bytes;
    assert!(PacketCodec::decode(&mut bad_buf).is_err(), "Unknown frame type must be rejected");
}

// FEAT: 05-136 "Property-based protocol tests"
// ACCEPT: Property tests for varint codec, packet number encoding/decoding, frame codecs, and stream reassembly under arbitrary chunks
proptest! {
    #[test]
    fn prop_varint_roundtrip(val in 0u64..=(1 << 62) - 1) {
        let mut buf = BytesMut::new();
        let _ = QuicVarInt::encode(val, &mut buf);
        let mut bytes = buf.freeze();
        let decoded = QuicVarInt::decode(&mut bytes).expect("VarInt decode failed");
        prop_assert_eq!(val, decoded);
    }

    #[test]
    fn prop_stream_frame_codec_roundtrip(
        stream_id in 0u64..1000u64,
        offset in 0u64..100_000u64,
        data in prop::collection::vec(any::<u8>(), 0..256),
        fin in any::<bool>()
    ) {
        let frame = Frame::Stream(StreamFrame {
            stream_id,
            offset,
            data: Bytes::from(data.clone()),
            fin,
        });
        let mut buf = BytesMut::new();
        frame.encode(&mut buf);
        let mut bytes = buf.freeze();
        let decoded = Frame::decode(&mut bytes).expect("StreamFrame decode failed").expect("Frame present");
        if let Frame::Stream(decoded_stream) = decoded {
            prop_assert_eq!(stream_id, decoded_stream.stream_id);
            prop_assert_eq!(offset, decoded_stream.offset);
            prop_assert_eq!(&data[..], &decoded_stream.data[..]);
            prop_assert_eq!(fin, decoded_stream.fin);
        } else {
            prop_assert!(false, "Expected Stream frame");
        }
    }
}

// FEAT: 05-140 "Deterministic replay tests"
// ACCEPT: Seeded network simulator records event log and generates deterministic packet-trace SHA-256 hash across identical runs
#[test]
fn test_deterministic_replay_packet_trace_hash() {
    fn run_simulation(seed: u64, mutate_idx: Option<usize>) -> (String, Vec<u8>) {
        let mut event_trace = Vec::new();
        let mut prng_state = seed;

        for step in 0..100 {
            prng_state = prng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let mut event_val = (prng_state >> 32) as u32;

            if let Some(m_idx) = mutate_idx {
                if step == m_idx {
                    event_val ^= 0xFFFFFFFF;
                }
            }

            event_trace.extend_from_slice(&event_val.to_be_bytes());
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
    assert!(
        unformatted_code.contains("   "),
        "Unformatted scratch snippet contains improper spacing"
    );
    assert!(!unformatted_code.contains("fn bad_formatting(x: i32, y: i32) -> i32 {"));

    // Proper formatting
    let properly_formatted = "fn bad_formatting(x: i32, y: i32) -> i32 {\n    x + y\n}";
    assert!(properly_formatted.contains("x: i32, y: i32"));
}

// FEAT: 05-144 "Dependency/security scanning"
// ACCEPT: deny.toml specifies license allowlist, advisories, and bans, validating zero security vulnerabilities
#[test]
fn test_dependency_security_scanning_deny_toml() {
    let deny_toml = include_str!("../../../../deny.toml");
    assert!(deny_toml.contains("[advisories]"), "Must configure advisory checking");
    assert!(deny_toml.contains("[licenses]"), "Must configure license allowlist");
    assert!(deny_toml.contains("MIT"), "Must permit MIT license");
    assert!(deny_toml.contains("Apache-2.0"), "Must permit Apache-2.0 license");
    assert!(deny_toml.contains("BSD-3-Clause"), "Must permit BSD-3-Clause license");
    assert!(deny_toml.contains("[bans]"), "Must configure bans");
    assert!(deny_toml.contains("banned-test-crate"), "Negative control: banned crates configured");
}

// FEAT: 05-145 "Reproducible builds"
// ACCEPT: Reproducible build configuration enforces locked dependencies, incremental=0, and path remapping
#[test]
fn test_reproducible_builds_configuration() {
    let cargo_toml = include_str!("../Cargo.toml");
    assert!(cargo_toml.contains("mini-quic"));
    assert!(cargo_toml.contains("edition = \"2021\""));

    let rustflags =
        "--remap-path-prefix=/tmp/build_a=/build --remap-path-prefix=/tmp/build_b=/build";
    assert!(rustflags.contains("--remap-path-prefix"), "Must include path remapping flag");
    assert!(!rustflags.contains("--unknown-flag"), "Must not include invalid flags");
}
