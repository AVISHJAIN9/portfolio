use bytes::{Bytes, BytesMut};
use mini_quic::wire::frame::{AckFrame, AckRange, CryptoFrame, Frame, StreamFrame};

// FEAT: 05-006 "Frame type registry"
// ACCEPT: Registry and serialization for core QUIC frame types (PING, PADDING, ACK, RESET, STREAM)
#[test]
fn test_frame_type_registry() {
    let mut buf = BytesMut::new();
    let frame = Frame::Ping;
    frame.encode(&mut buf);
    let mut read_buf = buf.freeze();
    let decoded = Frame::decode(&mut read_buf).unwrap().unwrap();
    assert_eq!(decoded, Frame::Ping);
}

// FEAT: 05-007 "STREAM frame encoding/decoding"
// ACCEPT: Encode and decode STREAM frames with stream ID, offset, data payload and FIN flag
#[test]
fn test_stream_frame_codec() {
    let mut buf = BytesMut::new();
    let stream = Frame::Stream(StreamFrame {
        stream_id: 12,
        offset: 4096,
        fin: true,
        data: Bytes::from_static(b"STREAM_PAYLOAD"),
    });
    stream.encode(&mut buf);
    let mut read_buf = buf.freeze();
    let decoded = Frame::decode(&mut read_buf).unwrap().unwrap();
    match decoded {
        Frame::Stream(s) => {
            assert_eq!(s.stream_id, 12);
            assert_eq!(s.offset, 4096);
            assert!(s.fin);
            assert_eq!(s.data.as_ref(), b"STREAM_PAYLOAD");
        }
        _ => panic!("Expected Stream frame"),
    }
}

// FEAT: 05-008 "ACK frame encoding/decoding"
// ACCEPT: Encode and decode ACK frames with largest acknowledged, ack delay, and ack ranges
#[test]
fn test_ack_frame_codec() {
    let mut buf = BytesMut::new();
    let ack = Frame::Ack(AckFrame {
        largest_acknowledged: 100,
        ack_delay: 25,
        first_ack_range: 5,
        ack_ranges: vec![AckRange { gap: 2, ack_range_length: 3 }],
        ecn_counts: None,
    });
    ack.encode(&mut buf);
    let mut read_buf = buf.freeze();
    let decoded = Frame::decode(&mut read_buf).unwrap().unwrap();
    match decoded {
        Frame::Ack(a) => {
            assert_eq!(a.largest_acknowledged, 100);
            assert_eq!(a.ack_delay, 25);
            assert_eq!(a.first_ack_range, 5);
            assert_eq!(a.ack_ranges.len(), 1);
        }
        _ => panic!("Expected Ack frame"),
    }
}

// FEAT: 05-009 "CRYPTO frame handling"
// ACCEPT: Handle CRYPTO frames containing TLS handshake tokens across packet number spaces
#[test]
fn test_crypto_frame_handling() {
    let mut buf = BytesMut::new();
    let crypto = Frame::Crypto(CryptoFrame {
        offset: 500,
        data: Bytes::from_static(b"TLS_HANDSHAKE_MESSAGE"),
    });
    crypto.encode(&mut buf);
    let mut read_buf = buf.freeze();
    let decoded = Frame::decode(&mut read_buf).unwrap().unwrap();
    match decoded {
        Frame::Crypto(c) => {
            assert_eq!(c.offset, 500);
            assert_eq!(c.data.as_ref(), b"TLS_HANDSHAKE_MESSAGE");
        }
        _ => panic!("Expected Crypto frame"),
    }
}

// FEAT: 05-010 "PING/PADDING/CONNECTION_CLOSE frames"
// ACCEPT: Encode and decode PING, PADDING, and CONNECTION_CLOSE control frames
#[test]
fn test_control_frames_codec() {
    for frame in [
        Frame::Padding { length: 8 },
        Frame::Ping,
        Frame::ConnectionClose {
            error_code: 0x01,
            frame_type: None,
            reason_phrase: "Normal closure".to_string(),
        },
    ] {
        let mut buf = BytesMut::new();
        frame.encode(&mut buf);
        let mut read_buf = buf.freeze();
        let decoded = Frame::decode(&mut read_buf).unwrap().unwrap();
        match (&frame, decoded) {
            (Frame::Padding { length }, Frame::Padding { length: l2 }) => assert_eq!(*length, l2),
            (Frame::Ping, Frame::Ping) => (),
            (Frame::ConnectionClose { error_code, .. }, Frame::ConnectionClose { error_code: e2, .. }) => assert_eq!(*error_code, e2),
            _ => panic!("Mismatch on control frame codec"),
        }
    }
}

// FEAT: 05-011 "MAX_DATA/MAX_STREAM_DATA frames"
// ACCEPT: Encode and decode MAX_DATA and MAX_STREAM_DATA connection flow control frames
#[test]
fn test_max_data_and_max_stream_data_frames() {
    let mut buf = BytesMut::new();
    let max_data_frame = Frame::MaxData { max_data: 1048576 };
    max_data_frame.encode(&mut buf);

    let max_stream_data_frame = Frame::MaxStreamData {
        stream_id: 4,
        max_stream_data: 262144,
    };
    max_stream_data_frame.encode(&mut buf);

    let mut read_buf = buf.freeze();
    let d1 = Frame::decode(&mut read_buf).unwrap().unwrap();
    assert_eq!(d1, Frame::MaxData { max_data: 1048576 });

    let d2 = Frame::decode(&mut read_buf).unwrap().unwrap();
    assert_eq!(
        d2,
        Frame::MaxStreamData {
            stream_id: 4,
            max_stream_data: 262144,
        }
    );
}

// FEAT: 05-012 "MAX_STREAMS frames"
// ACCEPT: Encode and decode MAX_STREAMS frames for bidirectional and unidirectional streams
#[test]
fn test_max_streams_frames() {
    let mut buf = BytesMut::new();
    let max_bidi = Frame::MaxStreams {
        is_bidi: true,
        max_streams: 128,
    };
    max_bidi.encode(&mut buf);

    let max_uni = Frame::MaxStreams {
        is_bidi: false,
        max_streams: 64,
    };
    max_uni.encode(&mut buf);

    let mut read_buf = buf.freeze();
    let d1 = Frame::decode(&mut read_buf).unwrap().unwrap();
    assert_eq!(d1, Frame::MaxStreams { is_bidi: true, max_streams: 128 });

    let d2 = Frame::decode(&mut read_buf).unwrap().unwrap();
    assert_eq!(d2, Frame::MaxStreams { is_bidi: false, max_streams: 64 });
}
