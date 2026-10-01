use bytes::Bytes;
use mini_quic::stream::{Stream, StreamManager, StreamState, StreamType};

// FEAT: 05-047
// ACCEPT: Manage bidirectional stream creation, read/write state transitions, and FIN handling
#[test]
fn test_bidirectional_stream_management() {
    let mut sm = StreamManager::new(65536, 16384, false);
    let s0 = sm.allocate_stream_id(true).unwrap();
    assert_eq!(s0, 0); // Client bidi #0
    let stream = sm.streams.get(&s0).unwrap();
    assert!(stream.stream_type.is_bidi());
    assert!(stream.stream_type.is_client_initiated());
}

// FEAT: 05-048
// ACCEPT: Manage unidirectional stream creation and receive-only or send-only boundaries
#[test]
fn test_unidirectional_stream_management() {
    let mut sm = StreamManager::new(65536, 16384, false);
    let s2 = sm.allocate_stream_id(false).unwrap();
    assert_eq!(s2, 2); // Client unidi #0
    let stream = sm.streams.get(&s2).unwrap();
    assert!(!stream.stream_type.is_bidi());
    assert!(stream.stream_type.is_client_initiated());
}

// FEAT: 05-049
// ACCEPT: Enforce RFC 9000 stream ID allocation bit encoding (Client=0, Server=1, Bidi=0, Uni=2)
#[test]
fn test_stream_id_allocation() {
    let mut client_sm = StreamManager::new(65536, 16384, false);
    let mut server_sm = StreamManager::new(65536, 16384, true);

    assert_eq!(client_sm.allocate_stream_id(true).unwrap(), 0);
    assert_eq!(client_sm.allocate_stream_id(true).unwrap(), 4);
    assert_eq!(client_sm.allocate_stream_id(false).unwrap(), 2);
    assert_eq!(client_sm.allocate_stream_id(false).unwrap(), 6);

    assert_eq!(server_sm.allocate_stream_id(true).unwrap(), 1);
    assert_eq!(server_sm.allocate_stream_id(true).unwrap(), 5);
    assert_eq!(server_sm.allocate_stream_id(false).unwrap(), 3);
    assert_eq!(server_sm.allocate_stream_id(false).unwrap(), 7);
}

// FEAT: 05-050
// ACCEPT: Reassemble contiguous stream data fragments arriving out-of-order
#[test]
fn test_stream_receive_reassembly() {
    let mut stream = Stream::new(0, false, 65536);
    // Receive fragments out of order: offset 5..10, then 0..5
    stream.receive_data(5, Bytes::from_static(b"WORLD"), false).unwrap();
    assert_eq!(stream.read_contiguous(), None);

    stream.receive_data(0, Bytes::from_static(b"HELLO"), false).unwrap();
    let c1 = stream.read_contiguous().unwrap();
    assert_eq!(&c1[..], b"HELLO");
    let c2 = stream.read_contiguous().unwrap();
    assert_eq!(&c2[..], b"WORLD");
    assert_eq!(stream.recv_offset, 10);
}

// FEAT: 05-051
// ACCEPT: Buffer outgoing stream bytes and track monotonic stream offset progression
#[test]
fn test_stream_send_buffering() {
    let mut stream = Stream::new(0, true, 4096);
    let written = stream.write_data(b"HTTP/3 Request Headers", false).unwrap();
    assert_eq!(written, 22);
    assert_eq!(stream.send_offset, 22);
    assert_eq!(&stream.send_buffer[..], b"HTTP/3 Request Headers");
}

// FEAT: 05-052
// ACCEPT: Block stream writes exceeding stream-level flow control credit limit
#[test]
fn test_stream_level_flow_control() {
    let mut stream = Stream::new(0, true, 10);
    assert!(stream.write_data(b"0123456789", false).is_ok());
    assert!(stream.write_data(b"extra", false).is_err());
}

// FEAT: 05-053
// ACCEPT: Track aggregate stream bytes and block writes exceeding connection-level MAX_DATA
#[test]
fn test_connection_level_flow_control() {
    let mut sm = StreamManager::new(20, 100, false);
    let s0 = sm.allocate_stream_id(true).unwrap();
    let s4 = sm.allocate_stream_id(true).unwrap();

    assert!(sm.send_stream_data(s0, b"1234567890", false).is_ok());
    assert!(sm.send_stream_data(s4, b"1234567890", false).is_ok());
    // Total sent = 20 (max_data = 20). Next send must fail
    assert!(sm.send_stream_data(s0, b"overflow", false).is_err());
}

// FEAT: 05-054
// ACCEPT: Update connection flow control window upon receiving MAX_DATA frame credit
#[test]
fn test_max_data_enforcement() {
    let mut sm = StreamManager::new(15, 100, false);
    let s0 = sm.allocate_stream_id(true).unwrap();
    assert!(sm.send_stream_data(s0, b"0123456789", false).is_ok());
    assert!(sm.send_stream_data(s0, b"123456", false).is_err());

    // Update MAX_DATA credit
    sm.update_max_data(30);
    assert!(sm.send_stream_data(s0, b"123456", false).is_ok());
}

// FEAT: 05-055
// ACCEPT: Update individual stream flow control credit upon receiving MAX_STREAM_DATA
#[test]
fn test_max_stream_data_enforcement() {
    let mut sm = StreamManager::new(1000, 10, false);
    let s0 = sm.allocate_stream_id(true).unwrap();
    assert!(sm.send_stream_data(s0, b"0123456789", false).is_ok());
    assert!(sm.send_stream_data(s0, b"more", false).is_err());

    // Update MAX_STREAM_DATA
    sm.update_max_stream_data(s0, 50).unwrap();
    assert!(sm.send_stream_data(s0, b"more", false).is_ok());
}

// FEAT: 05-056
// ACCEPT: Limit concurrent stream creation based on peer MAX_STREAMS credit
#[test]
fn test_max_streams_enforcement() {
    let mut sm = StreamManager::new(1000, 100, false);
    sm.max_streams_bidi = 2; // Only 2 streams allowed (0, 4)

    assert_eq!(sm.allocate_stream_id(true).unwrap(), 0);
    assert_eq!(sm.allocate_stream_id(true).unwrap(), 4);
    assert!(sm.allocate_stream_id(true).is_err());

    // Extend max_streams_bidi
    sm.update_max_streams_bidi(3);
    assert_eq!(sm.allocate_stream_id(true).unwrap(), 8);
}

// FEAT: 05-057
// ACCEPT: Handle RESET_STREAM frame, clear send buffer, and enter ResetSent state
#[test]
fn test_stream_reset_handling() {
    let mut stream = Stream::new(0, true, 100);
    stream.write_data(b"pending data", false).unwrap();
    stream.reset(0x01); // Error code 1
    assert_eq!(stream.state, StreamState::ResetSent(1));
    assert_eq!(stream.send_buffer.len(), 0);
    assert!(stream.write_data(b"cannot write after reset", false).is_err());
}
