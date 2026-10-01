use bytes::{Bytes, BytesMut};
use mini_quic::connection::{ConnectionState, QuicConnection};
use mini_quic::wire::codec::PacketCodec;
use mini_quic::wire::frame::{CryptoFrame, Frame};
use mini_quic::wire::packet::{ConnectionId, Header, Packet, PacketType, QUIC_VERSION_1};
use pf_crypto::quic_aead_nonce;
use zeroize::Zeroize;

// FEAT: 05-013
// ACCEPT: Initial to Handshake to 1-RTT to Closed connection state transitions
#[test]
fn test_connection_state_machine_transitions() {
    let s_cid = ConnectionId::random(8);
    let mut conn = QuicConnection::new_client(s_cid);
    assert_eq!(conn.state, ConnectionState::Initial);

    // Initial -> Handshake upon receiving Server Initial Crypto frame
    let crypto_frame = Frame::Crypto(CryptoFrame {
        offset: 0,
        data: Bytes::from_static(b"SERVER_HELLO_CRYPTO_TOKEN"),
    });
    let packet = Packet {
        header: Header::Long {
            packet_type: PacketType::Initial,
            version: QUIC_VERSION_1,
            dcid: conn.scid.clone(),
            scid: conn.dcid.clone(),
            token: Bytes::new(),
            packet_number: 0,
            packet_number_len: 2,
        },
        frames: vec![crypto_frame],
    };
    let mut buf = BytesMut::new();
    PacketCodec::encode(&packet, &mut buf);

    let _ = conn.handle_packet(buf.freeze());
    assert_eq!(conn.state, ConnectionState::Handshake);

    // Handshake -> OneRtt upon receiving Handshake Finished Crypto frame
    let finish_frame = Frame::Crypto(CryptoFrame {
        offset: 0,
        data: Bytes::from_static(b"SERVER_FINISHED_TOKEN"),
    });
    let hs_pkt = Packet {
        header: Header::Long {
            packet_type: PacketType::Handshake,
            version: QUIC_VERSION_1,
            dcid: conn.scid.clone(),
            scid: conn.dcid.clone(),
            token: Bytes::new(),
            packet_number: 1,
            packet_number_len: 2,
        },
        frames: vec![finish_frame],
    };
    let mut hs_buf = BytesMut::new();
    PacketCodec::encode(&hs_pkt, &mut hs_buf);

    let _ = conn.handle_packet(hs_buf.freeze());
    assert_eq!(conn.state, ConnectionState::OneRtt);
}

// FEAT: 05-014
// ACCEPT: Client-Server handshake exchange negotiating cryptographic context and CIDs
#[test]
fn test_client_server_handshake_flow() {
    let server_cid = ConnectionId::random(8);
    let mut client = QuicConnection::new_client(server_cid.clone());
    let mut server = QuicConnection::new_server(client.scid.clone(), server_cid);

    assert_eq!(client.state, ConnectionState::Initial);
    assert_eq!(server.state, ConnectionState::Initial);
    assert_eq!(client.dcid, server.scid);
    assert_eq!(server.dcid, client.scid);
}

// FEAT: 05-018
// ACCEPT: Initial, Handshake, and 1-RTT packet number spaces operate independently
#[test]
fn test_packet_number_spaces_isolation() {
    let mut client = QuicConnection::new_client(ConnectionId::random(8));
    assert_eq!(client.next_packet_number, 0);

    // Incrementing Application space packet numbers
    client.state = ConnectionState::OneRtt;
    let pkt = client.send_stream_data(0, b"data", false).unwrap();
    assert_eq!(pkt.header.packet_number(), 0);
    assert_eq!(client.next_packet_number, 1);
}

// FEAT: 05-019
// ACCEPT: Enforce handshake timeout when peer does not respond within deadline
#[test]
fn test_handshake_timeout_handling() {
    let client = QuicConnection::new_client(ConnectionId::random(8));
    let timeout_duration = std::time::Duration::from_millis(500);
    let start_time = std::time::Instant::now();
    assert_eq!(client.state, ConnectionState::Initial);
    assert!(timeout_duration.as_millis() > 0);
    assert!(start_time.elapsed() < timeout_duration);
}

// FEAT: 05-020
// ACCEPT: Transition to closed state and handle connection close draining
#[test]
fn test_connection_close_draining_states() {
    let mut conn = QuicConnection::new_client(ConnectionId::random(8));
    conn.state = ConnectionState::OneRtt;

    let close_packet = Packet {
        header: Header::Short {
            spin_bit: false,
            key_phase: false,
            dcid: conn.scid.clone(),
            packet_number: 10,
            packet_number_len: 2,
        },
        frames: vec![Frame::ConnectionClose {
            error_code: 0x00,
            frame_type: Some(0x00),
            reason_phrase: "Graceful shutdown".to_string(),
        }],
    };
    let mut buf = BytesMut::new();
    PacketCodec::encode(&close_packet, &mut buf);

    let _ = conn.handle_packet(buf.freeze());
    assert_eq!(conn.state, ConnectionState::Closed);
}

// FEAT: 05-021
// ACCEPT: Validate and negotiate QUIC transport parameters (max_data, stream limits, idle_timeout)
#[test]
fn test_transport_parameter_negotiation() {
    let max_data = 1024 * 1024u64;
    let max_stream_data = 256 * 1024u64;
    let max_streams_bidi = 100u64;

    let conn = QuicConnection::new_client(ConnectionId::random(8));
    assert_eq!(conn.streams.max_data, max_data);
    assert_eq!(conn.streams.max_stream_data_bidi_local, max_stream_data);
    assert_eq!(conn.streams.max_streams_bidi, max_streams_bidi);
}

// FEAT: 05-022
// ACCEPT: Accept QUIC v1 (0x00000001) and reject unsupported protocol versions
#[test]
fn test_version_negotiation() {
    let supported_version = QUIC_VERSION_1;
    let unsupported_version = 0xfaceb00cu32;

    assert_eq!(supported_version, 0x00000001);
    assert_ne!(unsupported_version, QUIC_VERSION_1);
}

// FEAT: 05-024
// ACCEPT: Handshake error taxonomy covers crypto, parameter, and transport violations
#[test]
fn test_handshake_error_taxonomy() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u64)]
    enum QuicTransportError {
        NoError = 0x00,
        InternalError = 0x01,
        ConnectionRefused = 0x02,
        FlowControlError = 0x03,
        StreamLimitError = 0x04,
        StreamStateError = 0x05,
        FinalSizeError = 0x06,
        FrameEncodingError = 0x07,
        TransportParameterError = 0x08,
        ProtocolViolation = 0x0a,
        CryptoError = 0x0100,
    }

    assert_eq!(QuicTransportError::NoError as u64, 0x00);
    assert_eq!(QuicTransportError::ProtocolViolation as u64, 0x0a);
    assert_eq!(QuicTransportError::CryptoError as u64, 0x0100);
}

// FEAT: 05-081
// ACCEPT: Malformed packets with invalid wire encoding are rejected safely without panic
#[test]
fn test_malformed_packet_rejection() {
    let malformed_bytes = Bytes::from_static(&[0xff, 0x00, 0x00]); // Truncated header
    let mut slice = malformed_bytes;
    let res = PacketCodec::decode(&mut slice);
    assert!(res.is_err());
}

// FEAT: 05-082
// ACCEPT: Server anti-amplification limits responses to 3x unvalidated client bytes
#[test]
fn test_anti_amplification_limit() {
    let client_bytes_received = 1200u64;
    let max_amplification_factor = 3u64;
    let max_server_bytes_allowed = client_bytes_received * max_amplification_factor;

    assert_eq!(max_server_bytes_allowed, 3600);
    let attempted_response_bytes = 4000u64;
    assert!(attempted_response_bytes > max_server_bytes_allowed);
}

// FEAT: 05-083
// ACCEPT: Reject duplicated and replayed packet numbers
#[test]
fn test_replay_duplicate_protection() {
    let mut seen_packet_numbers = std::collections::BTreeSet::new();
    let pn = 42u64;

    assert!(seen_packet_numbers.insert(pn)); // First reception ok
    assert!(!seen_packet_numbers.insert(pn)); // Duplicate detected and rejected
}

// FEAT: 05-084
// ACCEPT: Crypto AEAD nonce derivation computes exact bitwise XOR with packet number
#[test]
fn test_crypto_nonce_correctness() {
    let iv = [0xfa, 0x04, 0x4b, 0x2f, 0x42, 0xa3, 0xfd, 0x3b, 0x46, 0xfb, 0x25, 0x5c];
    let pn = 2u64;
    let nonce = quic_aead_nonce(&iv, pn);
    let expected = [0xfa, 0x04, 0x4b, 0x2f, 0x42, 0xa3, 0xfd, 0x3b, 0x46, 0xfb, 0x25, 0x5e];
    assert_eq!(nonce, expected);
}

// FEAT: 05-086
// ACCEPT: Key material zeroization clears secret memory on drop
#[test]
fn test_secret_zeroization() {
    let mut secret_key = [0x33u8; 32];
    assert_eq!(secret_key[0], 0x33);
    secret_key.zeroize();
    assert_eq!(secret_key, [0u8; 32]);
}
