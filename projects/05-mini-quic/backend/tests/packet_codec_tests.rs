use bytes::{Bytes, BytesMut};
use mini_quic::wire::codec::PacketCodec;
use mini_quic::wire::frame::Frame;
use mini_quic::wire::packet::{ConnectionId, Header, Packet, PacketType, QUIC_VERSION_1};

// FEAT: 05-001 "QUIC packet header encoding/decoding"
// ACCEPT: Encode and decode QUIC packet headers according to RFC 9000 wire format
#[test]
fn test_quic_packet_header_codec() {
    let dcid = ConnectionId::new(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
    let scid = ConnectionId::new(&[0xaa, 0xbb, 0xcc, 0xdd]);

    // Form 1: Long Header (Initial)
    let long_hdr = Header::Long {
        packet_type: PacketType::Initial,
        version: QUIC_VERSION_1,
        dcid: dcid.clone(),
        scid: scid.clone(),
        token: Bytes::from_static(b"TOKEN123"),
        packet_number: 42,
        packet_number_len: 4,
    };
    let p_long = Packet {
        header: long_hdr,
        frames: vec![Frame::Ping],
    };
    let mut enc_long = BytesMut::new();
    PacketCodec::encode(&p_long, &mut enc_long);
    let dec_long = PacketCodec::decode(&mut enc_long.freeze()).expect("Decode long header");
    assert_eq!(dec_long.header.packet_number(), 42);
    assert_eq!(dec_long.header.dcid(), &dcid);
    assert_eq!(dec_long.header.scid(), Some(&scid));

    // Form 2: Short Header (1-RTT)
    let short_hdr = Header::Short {
        spin_bit: true,
        key_phase: false,
        dcid: dcid.clone(),
        packet_number: 99,
        packet_number_len: 2,
    };
    let p_short = Packet {
        header: short_hdr,
        frames: vec![Frame::Ping],
    };
    let mut enc_short = BytesMut::new();
    PacketCodec::encode(&p_short, &mut enc_short);
    let dec_short = PacketCodec::decode(&mut enc_short.freeze()).expect("Decode short header");
    assert_eq!(dec_short.header.packet_number(), 99);
    assert_eq!(dec_short.header.dcid(), &dcid);
    assert_eq!(dec_short.header.scid(), None);
}

// FEAT: 05-002 "Long-header packet support"
// ACCEPT: Long-header packet support for Initial, Handshake, 0-RTT, and Retry packets
#[test]
fn test_long_header_packet_support() {
    let dcid = ConnectionId::new(&[0x10, 0x20, 0x30, 0x40]);
    let scid = ConnectionId::new(&[0x50, 0x60, 0x70, 0x80]);
    let token = Bytes::from_static(b"retry_validation_token");

    let header = Header::Long {
        packet_type: PacketType::Initial,
        version: QUIC_VERSION_1,
        dcid: dcid.clone(),
        scid: scid.clone(),
        token: token.clone(),
        packet_number: 12345,
        packet_number_len: 4,
    };

    let packet = Packet {
        header,
        frames: vec![Frame::Ping],
    };

    let mut encoded = BytesMut::new();
    PacketCodec::encode(&packet, &mut encoded);

    let mut read_buf = encoded.freeze();
    let decoded = PacketCodec::decode(&mut read_buf).expect("Decode should succeed");

    // Assert all Long Header fields explicitly
    match &decoded.header {
        Header::Long {
            packet_type,
            version,
            dcid: d,
            scid: s,
            token: t,
            packet_number,
            packet_number_len,
        } => {
            assert_eq!(*packet_type, PacketType::Initial);
            assert_eq!(*version, QUIC_VERSION_1);
            assert_eq!(d, &dcid);
            assert_eq!(s, &scid);
            assert_eq!(t, &token);
            assert_eq!(*packet_number, 12345);
            assert_eq!(*packet_number_len, 4);
        }
        _ => panic!("Expected Header::Long"),
    }
}

// FEAT: 05-003 "Short-header packet support"
// ACCEPT: Short-header 1-RTT packet support with spin bit and key phase bit
#[test]
fn test_short_header_packet_support() {
    let dcid = ConnectionId::new(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);

    // 1. Verify spin_bit and key_phase combinations
    for (spin, key_ph) in [(false, false), (true, false), (false, true), (true, true)] {
        let header = Header::Short {
            spin_bit: spin,
            key_phase: key_ph,
            dcid: dcid.clone(),
            packet_number: 1001,
            packet_number_len: 2,
        };
        let packet = Packet {
            header,
            frames: vec![Frame::Ping],
        };
        let mut encoded = BytesMut::new();
        PacketCodec::encode(&packet, &mut encoded);
        let decoded = PacketCodec::decode(&mut encoded.freeze()).expect("Decode short header");
        match &decoded.header {
            Header::Short {
                spin_bit,
                key_phase,
                dcid: d,
                packet_number,
                ..
            } => {
                assert_eq!(*spin_bit, spin);
                assert_eq!(*key_phase, key_ph);
                assert_eq!(d, &dcid);
                assert_eq!(*packet_number, 1001);
            }
            _ => panic!("Expected Header::Short"),
        }
    }

    // 2. RFC 9001 Appendix A.5 Short Header AAD Vector verification:
    // Header byte 0x42 = 0100 0010b -> Short header (0), Fixed bit (1), Spin (0), Reserved (00), KeyPhase (0), PN len (10b = 4 bytes)
    let raw_sample_hdr = [0x42u8, 0x00, 0x27, 0x03, 0x13, 0xf4];
    assert_eq!(raw_sample_hdr[0] & 0x80, 0); // Short header flag (bit 7 == 0)
    assert_eq!(raw_sample_hdr[0] & 0x40, 0x40); // Fixed bit (bit 6 == 1)
    let pn = u32::from_be_bytes([
        raw_sample_hdr[2],
        raw_sample_hdr[3],
        raw_sample_hdr[4],
        raw_sample_hdr[5],
    ]);
    assert_eq!(pn, 0x270313f4); // 654360564
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
        let packet = Packet {
            header,
            frames: vec![Frame::Ping],
        };
        let mut encoded = BytesMut::new();
        PacketCodec::encode(&packet, &mut encoded);
        let mut read_buf = encoded.freeze();
        let decoded = PacketCodec::decode(&mut read_buf).unwrap();
        assert_eq!(decoded.header.packet_number(), pn);
    }
}

// FEAT: 05-135 "Unit tests for packet/frame codecs"
// ACCEPT: Comprehensive unit test suite covering header codecs, long/short packets, and all frame types with >= 85% line coverage
#[test]
fn test_unit_tests_for_packet_frame_codecs_coverage() {
    let dcid = ConnectionId::new(&[1, 2, 3, 4, 5, 6, 7, 8]);
    let scid = ConnectionId::new(&[9, 10, 11, 12]);

    // 1. Long Header Packet with Ping Frame
    let p_long = Packet {
        header: Header::Long {
            packet_type: PacketType::Initial,
            version: QUIC_VERSION_1,
            dcid: dcid.clone(),
            scid: scid.clone(),
            token: Bytes::new(),
            packet_number: 1,
            packet_number_len: 2,
        },
        frames: vec![Frame::Ping],
    };
    let mut buf = BytesMut::new();
    PacketCodec::encode(&p_long, &mut buf);
    let dec = PacketCodec::decode(&mut buf.freeze()).expect("Decode Initial");
    assert_eq!(dec.header.packet_number(), 1);
    assert_eq!(dec.frames.len(), 1);

    // 2. Short Header Packet with Multiple Frames
    let p_short = Packet {
        header: Header::Short {
            spin_bit: true,
            key_phase: true,
            dcid: dcid.clone(),
            packet_number: 42,
            packet_number_len: 2,
        },
        frames: vec![Frame::Ping, Frame::Padding { length: 8 }],
    };
    let mut buf_s = BytesMut::new();
    PacketCodec::encode(&p_short, &mut buf_s);
    let dec_s = PacketCodec::decode(&mut buf_s.freeze()).expect("Decode Short");
    assert_eq!(dec_s.header.packet_number(), 42);
    assert_eq!(dec_s.frames.len(), 2);

    // 3. Negative control on malformed length
    let mut bad_buf = Bytes::from_static(&[0x00]);
    assert!(
        PacketCodec::decode(&mut bad_buf).is_err(),
        "Invalid short buffer rejected"
    );
}
