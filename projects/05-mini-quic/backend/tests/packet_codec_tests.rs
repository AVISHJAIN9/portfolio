use bytes::{Bytes, BytesMut};
use mini_quic::wire::codec::PacketCodec;
use mini_quic::wire::frame::{AckFrame, CryptoFrame, Frame, StreamFrame};
use mini_quic::wire::packet::{ConnectionId, Header, Packet, PacketType, QUIC_VERSION_1};

#[test]
fn test_long_header_initial_packet_roundtrip() {
    let dcid = ConnectionId::new(b"server_cid_123");
    let scid = ConnectionId::new(b"client_cid_456");

    let header = Header::Long {
        packet_type: PacketType::Initial,
        version: QUIC_VERSION_1,
        dcid,
        scid,
        token: Bytes::from_static(b"token_sample"),
        packet_number: 1,
        packet_number_len: 2,
    };

    let crypto_frame = Frame::Crypto(CryptoFrame {
        offset: 0,
        data: Bytes::from_static(b"TLS_CLIENT_HELLO_BYTES"),
    });

    let packet = Packet {
        header,
        frames: vec![crypto_frame, Frame::Ping],
    };

    let mut encoded = BytesMut::new();
    PacketCodec::encode(&packet, &mut encoded);

    let mut read_buf = encoded.freeze();
    let decoded = PacketCodec::decode(&mut read_buf).expect("Failed to decode packet");

    assert_eq!(decoded.header.packet_number(), 1);
    assert_eq!(decoded.frames.len(), 2);
    assert!(matches!(decoded.frames[0], Frame::Crypto(_)));
    assert_eq!(decoded.frames[1], Frame::Ping);
}

#[test]
fn test_short_header_1rtt_stream_packet_roundtrip() {
    let dcid = ConnectionId::new(b"12345678");
    let header = Header::Short {
        spin_bit: true,
        key_phase: false,
        dcid,
        packet_number: 42,
        packet_number_len: 2,
    };

    let stream_frame = Frame::Stream(StreamFrame {
        stream_id: 4,
        offset: 1024,
        fin: true,
        data: Bytes::from_static(b"HTTP/3 Request Payload Data"),
    });

    let ack_frame = Frame::Ack(AckFrame {
        largest_acknowledged: 40,
        ack_delay: 2,
        first_ack_range: 0,
        ack_ranges: vec![],
        ecn_counts: None,
    });

    let packet = Packet {
        header,
        frames: vec![stream_frame, ack_frame],
    };

    let mut encoded = BytesMut::new();
    PacketCodec::encode(&packet, &mut encoded);

    let mut read_buf = encoded.freeze();
    let decoded = PacketCodec::decode(&mut read_buf).expect("Failed to decode short packet");

    assert_eq!(decoded.header.packet_number(), 42);
    assert_eq!(decoded.frames.len(), 2);
}

#[test]
fn test_corrupted_checksum_rejection() {
    let dcid = ConnectionId::new(b"12345678");
    let header = Header::Short {
        spin_bit: false,
        key_phase: false,
        dcid,
        packet_number: 10,
        packet_number_len: 2,
    };

    let packet = Packet {
        header,
        frames: vec![Frame::Ping],
    };

    let mut encoded = BytesMut::new();
    PacketCodec::encode(&packet, &mut encoded);

    // Corrupt one byte in the body
    let last_idx = encoded.len() - 5;
    encoded[last_idx] ^= 0xFF;

    let mut read_buf = encoded.freeze();
    let result = PacketCodec::decode(&mut read_buf);
    assert!(result.is_err());
}
