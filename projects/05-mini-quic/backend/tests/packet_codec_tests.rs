use bytes::{Bytes, BytesMut};
use mini_quic::wire::codec::PacketCodec;
use mini_quic::wire::frame::{CryptoFrame, Frame, StreamFrame};
use mini_quic::wire::packet::{ConnectionId, Header, Packet, PacketType, QUIC_VERSION_1};

// FEAT: 05-001 "QUIC packet header encoding/decoding"
// ACCEPT: Encode and decode QUIC packet headers according to RFC 9000 wire format
#[test]
fn test_quic_packet_header_codec() {
    let dcid = ConnectionId::new(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
    let scid = ConnectionId::new(&[0xaa, 0xbb, 0xcc, 0xdd]);
    
    let header = Header::Long {
        packet_type: PacketType::Initial,
        version: QUIC_VERSION_1,
        dcid,
        scid,
        token: Bytes::new(),
        packet_number: 42,
        packet_number_len: 4,
    };

    let packet = Packet {
        header,
        frames: vec![
            Frame::Ping,
            Frame::Crypto(CryptoFrame {
                offset: 0,
                data: Bytes::from_static(b"CLIENT_HELLO_PAYLOAD"),
            }),
        ],
    };

    let mut encoded = BytesMut::new();
    PacketCodec::encode(&packet, &mut encoded);

    let mut read_buf = encoded.freeze();
    let decoded = PacketCodec::decode(&mut read_buf).expect("Decode should succeed");

    assert_eq!(decoded.header.packet_number(), 42);
    assert_eq!(decoded.frames.len(), 2);
}

// FEAT: 05-002 "Long-header packet support"
// ACCEPT: Long-header packet support for Initial, Handshake, 0-RTT, and Retry packets
#[test]
fn test_long_header_packet_support() {
    let dcid = ConnectionId::new(&[0x10, 0x20, 0x30, 0x40]);
    let scid = ConnectionId::new(&[0x50, 0x60, 0x70, 0x80]);
    let header = Header::Long {
        packet_type: PacketType::Handshake,
        version: QUIC_VERSION_1,
        dcid: dcid.clone(),
        scid,
        token: Bytes::new(),
        packet_number: 1,
        packet_number_len: 2,
    };

    let packet = Packet {
        header,
        frames: vec![Frame::Ping],
    };

    let mut encoded = BytesMut::new();
    PacketCodec::encode(&packet, &mut encoded);

    let mut read_buf = encoded.freeze();
    let decoded = PacketCodec::decode(&mut read_buf).expect("Decode should succeed");
    assert_eq!(decoded.header.dcid(), &dcid);
}

// FEAT: 05-003 "Short-header packet support"
// ACCEPT: Short-header 1-RTT packet support with spin bit and key phase bit
#[test]
fn test_short_header_packet_support() {
    let dcid = ConnectionId::new(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    let header = Header::Short {
        spin_bit: false,
        key_phase: false,
        dcid: dcid.clone(),
        packet_number: 1001,
        packet_number_len: 2,
    };

    let packet = Packet {
        header,
        frames: vec![
            Frame::Stream(StreamFrame {
                stream_id: 4,
                offset: 1024,
                data: Bytes::from_static(b"STREAM_PAYLOAD_CHUNK_DATA"),
                fin: true,
            }),
        ],
    };

    let mut encoded = BytesMut::new();
    PacketCodec::encode(&packet, &mut encoded);

    let mut read_buf = encoded.freeze();
    let decoded = PacketCodec::decode(&mut read_buf).expect("Decode should succeed");

    assert_eq!(decoded.header.packet_number(), 1001);
    assert_eq!(decoded.header.dcid(), &dcid);
}

// FEAT: 05-004 "Connection ID management"
// ACCEPT: Connection ID generation, routing, and zero-length to 20-byte CID support
#[test]
fn test_connection_id_management() {
    let cid1 = ConnectionId::new(&[1, 2, 3, 4]);
    let cid2 = ConnectionId::random(8);
    assert_eq!(cid1.0.len(), 4);
    assert_eq!(cid2.0.len(), 8);
    assert_ne!(cid1, cid2);
}

// FEAT: 05-005 "Packet number encoding/decoding"
// ACCEPT: Encode and decode truncated 1, 2, and 4-byte packet numbers per RFC 9000
#[test]
fn test_packet_number_encoding_decoding() {
    let dcid = ConnectionId::new(&[0xaa; 8]);
    for (pn, len) in [(100u64, 1), (5000u64, 2), (1000000u64, 4)] {
        let header = Header::Short {
            spin_bit: false,
            key_phase: false,
            dcid: dcid.clone(),
            packet_number: pn,
            packet_number_len: len,
        };
        let packet = Packet { header, frames: vec![Frame::Ping] };
        let mut encoded = BytesMut::new();
        PacketCodec::encode(&packet, &mut encoded);
        let mut read_buf = encoded.freeze();
        let decoded = PacketCodec::decode(&mut read_buf).unwrap();
        assert_eq!(decoded.header.packet_number(), pn);
    }
}
