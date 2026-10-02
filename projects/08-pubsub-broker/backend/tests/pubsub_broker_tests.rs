//! Test suite for Project 08: PubSub Broker
//! Covers features 08-001..08-117

use pubsub_broker::delivery::DeliveryCoordinator;
use pubsub_broker::groups::GroupCoordinator;
use pubsub_broker::log::PartitionLog;
use pubsub_broker::model::Record;
use pubsub_broker::producer::{IdempotentProducerRegistry, ProducerError};
use pubsub_broker::raft::{RaftReplica, ReplicaRole};
use pubsub_broker::retention::Compactor;
use pubsub_broker::topics::TopicRegistry;
use pubsub_broker::wal::{FsyncPolicy, WriteAheadLog};
use std::collections::HashMap;
use tempfile::TempDir;

/// [FEAT: 08-001, 08-002, 08-034] Record Binary Encoding, Decoding & CRC32C Integrity
#[test]
fn test_record_encoding_decoding_and_crc() {
    let mut headers = HashMap::new();
    headers.insert("env".to_string(), "production".to_string());

    let record = Record::new(100, 1690000000, Some(b"sensor-1".to_vec()), b"payload-bytes".to_vec(), headers);
    assert!(record.is_valid());

    let encoded = record.encode();
    assert!(encoded.len() > 20);

    let decoded = Record::decode(encoded.clone()).expect("decode valid record");
    assert_eq!(decoded.offset, 100);
    assert_eq!(decoded.key, Some(b"sensor-1".to_vec()));
    assert_eq!(decoded.value, b"payload-bytes".to_vec());
    assert_eq!(decoded.headers.get("env").map(|s| s.as_str()), Some("production"));

    // Negative: Corrupted CRC32
    let mut corrupted = encoded.to_vec();
    corrupted[0] ^= 0xFF; // flip bits in offset
    assert!(Record::decode(bytes::Bytes::from(corrupted)).is_err());
}

/// [FEAT: 08-005, 08-006, 08-014, 08-017, 08-018, 08-024] Topic Management & Routing
#[test]
fn test_topic_management_and_partition_routing() {
    let registry = TopicRegistry::new();

    // Create topic
    assert!(registry.create_topic("telemetry".to_string(), 4, 3, 86400000).is_ok());
    // Negative: Duplicate topic creation fails
    assert!(registry.create_topic("telemetry".to_string(), 4, 3, 86400000).is_err());
    // Negative: 0 partition count fails
    assert!(registry.create_topic("invalid".to_string(), 0, 1, 86400000).is_err());

    // Deterministic key routing
    let p_a1 = registry.route_partition("telemetry", Some(b"device-alpha")).unwrap();
    let p_a2 = registry.route_partition("telemetry", Some(b"device-alpha")).unwrap();
    assert_eq!(p_a1, p_a2);
    assert!(p_a1 < 4);

    // Round robin routing
    let p_rr1 = registry.route_partition("telemetry", None).unwrap();
    let p_rr2 = registry.route_partition("telemetry", None).unwrap();
    let p_rr3 = registry.route_partition("telemetry", None).unwrap();
    let p_rr4 = registry.route_partition("telemetry", None).unwrap();
    assert_eq!(p_rr1, 0);
    assert_eq!(p_rr2, 1);
    assert_eq!(p_rr3, 2);
    assert_eq!(p_rr4, 3);

    // Hot partition detection
    for _ in 0..100 {
        registry.record_partition_message("telemetry", 2);
    }
    for _ in 0..5 {
        registry.record_partition_message("telemetry", 0);
    }
    assert!(registry.is_hot_partition("telemetry", 2, 2.0));
    assert!(!registry.is_hot_partition("telemetry", 0, 2.0));

    // Delete topic
    assert!(registry.delete_topic("telemetry"));
    assert!(registry.get_topic("telemetry").is_none());
}

/// [FEAT: 08-027, 08-028, 08-029, 08-030, 08-031, 08-037] Append-Only Log & Segment Rolling
#[test]
fn test_partition_log_segments_and_index_search() {
    let tmp = TempDir::new().unwrap();
    // Use small max_segment_size (300 bytes) to force rolling
    let log = PartitionLog::new(tmp.path(), 300, 2).expect("init log");

    for i in 0..10 {
        let rec = log.append(
            Some(format!("k-{}", i).into_bytes()),
            format!("value-content-for-index-{}", i).into_bytes(),
            HashMap::new(),
            1000 + i as i64,
        ).unwrap();
        assert_eq!(rec.offset, i);
    }

    assert_eq!(log.next_offset(), 10);
    assert!(log.verify_integrity().unwrap());

    // Read range
    let read_records = log.read_from(3, 4).unwrap();
    assert_eq!(read_records.len(), 4);
    assert_eq!(read_records[0].offset, 3);
    assert_eq!(read_records[3].offset, 6);

    // Negative: Read past end of log returns empty
    let empty_read = log.read_from(100, 5).unwrap();
    assert!(empty_read.is_empty());
}

/// [FEAT: 08-040, 08-041, 08-044, 08-045, 08-046, 08-048] WAL Group Commit & Recovery
#[test]
fn test_wal_group_commit_and_crash_recovery() {
    let tmp = TempDir::new().unwrap();
    let wal_path = tmp.path().join("wal_test.log");

    let wal = WriteAheadLog::open(&wal_path, FsyncPolicy::GroupCommit { batch_size: 3, timeout_ms: 50 }).unwrap();

    let rec1 = Record::new(0, 100, None, b"wal-entry-0".to_vec(), HashMap::new());
    let rec2 = Record::new(1, 101, None, b"wal-entry-1".to_vec(), HashMap::new());
    let rec3 = Record::new(2, 102, None, b"wal-entry-2".to_vec(), HashMap::new());

    wal.write_entry(&rec1).unwrap();
    wal.write_entry(&rec2).unwrap();
    assert_eq!(wal.flushed_offset(), 0); // Not flushed yet

    wal.write_entry(&rec3).unwrap(); // Triggers batch flush of 3
    assert_eq!(wal.flushed_offset(), 2);

    // Replay WAL
    let (replayed, corrupted) = wal.replay_and_recover().unwrap();
    assert_eq!(replayed.len(), 3);
    assert_eq!(corrupted, 0);

    // Fault injection: Append partial corrupted bytes to WAL file
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().append(true).open(&wal_path).unwrap();
        f.write_all(&[0xDE, 0xAD, 0xBE, 0xEF]).unwrap();
    }

    // Recover should detect and truncate partial corrupted bytes
    let (replayed_after_crash, truncated_bytes) = wal.replay_and_recover().unwrap();
    assert_eq!(replayed_after_crash.len(), 3);
    assert_eq!(truncated_bytes, 4);
}

/// [FEAT: 08-053, 08-054, 08-056, 08-057, 08-059] Raft-Lite Replication & Quorum High Watermark
#[test]
fn test_raft_replication_and_quorum_advancement() {
    let replica = RaftReplica::new(1, &[2, 3]); // 3-node cluster, quorum = 2
    assert_eq!(replica.role(), ReplicaRole::Follower);

    // Election
    replica.start_election();
    assert_eq!(replica.role(), ReplicaRole::Candidate);
    assert_eq!(replica.current_term(), 1);

    replica.become_leader();
    assert_eq!(replica.role(), ReplicaRole::Leader);

    // Leader appends locally up to LEO 10
    replica.record_local_append(10);
    assert_eq!(replica.high_watermark(), 0); // Follower 2 and 3 are at 0

    // Follower 2 acknowledges up to LEO 8 (Quorum 2/3 achieved for offset 8)
    let hw = replica.update_follower_offset(2, 8);
    assert_eq!(hw, 8);
    assert_eq!(replica.high_watermark(), 8);

    // Follower 3 acknowledges up to LEO 10 (Quorum 2/3 achieved for offset 10)
    let hw2 = replica.update_follower_offset(3, 10);
    assert_eq!(hw2, 10);
    assert_eq!(replica.high_watermark(), 10);

    // Heartbeat reception on follower node
    let follower_node = RaftReplica::new(2, &[1, 3]);
    assert!(follower_node.handle_heartbeat(1, 1, 10));
    assert_eq!(follower_node.current_term(), 1);
    // Negative: Stale term heartbeat rejected
    assert!(!follower_node.handle_heartbeat(1, 0, 10));
}

/// [FEAT: 08-066, 08-067, 08-068, 08-069, 08-070] Idempotent Producer & Sequence Tracking
#[test]
fn test_idempotent_producer_sequence_tracking() {
    let registry = IdempotentProducerRegistry::new();
    registry.init_producer(1001, 1);

    // Valid sequential writes
    assert!(registry.verify_and_advance_sequence(1001, 1, 1).is_ok());
    assert!(registry.verify_and_advance_sequence(1001, 1, 2).is_ok());
    assert!(registry.verify_and_advance_sequence(1001, 1, 3).is_ok());
    assert_eq!(registry.last_sequence(1001), Some(3));

    // Negative: Duplicate sequence
    let dup_res = registry.verify_and_advance_sequence(1001, 1, 2);
    assert!(matches!(dup_res, Err(ProducerError::DuplicateSequence { sequence: 2, .. })));

    // Negative: Out-of-order sequence gap (expected 4, got 6)
    let gap_res = registry.verify_and_advance_sequence(1001, 1, 6);
    assert!(matches!(gap_res, Err(ProducerError::SequenceOutOfOrder { expected: 4, got: 6, .. })));

    // Negative: Fenced older epoch
    let fenced_res = registry.verify_and_advance_sequence(1001, 0, 4);
    assert!(matches!(fenced_res, Err(ProducerError::FencedEpoch { current_epoch: 1, got_epoch: 0, .. })));
}

/// [FEAT: 08-079, 08-080, 08-081, 08-082, 08-009] Consumer Groups & Cooperative Rebalance
#[test]
fn test_consumer_group_cooperative_rebalance() {
    let coordinator = GroupCoordinator::new();

    coordinator.join_group("billing-cg", "invoices", "consumer-A", "10.0.0.1", 100);
    coordinator.join_group("billing-cg", "invoices", "consumer-B", "10.0.0.2", 100);

    // Rebalance 6 partitions across 2 members
    let gen = coordinator.rebalance("billing-cg", 6).unwrap();
    assert_eq!(gen, 2);

    let parts_a = coordinator.get_member_assignments("billing-cg", "consumer-A").unwrap();
    let parts_b = coordinator.get_member_assignments("billing-cg", "consumer-B").unwrap();
    assert_eq!(parts_a.len(), 3);
    assert_eq!(parts_b.len(), 3);

    // Offset commit
    assert!(coordinator.commit_offset("billing-cg", 0, 42).is_ok());
    assert_eq!(coordinator.get_committed_offset("billing-cg", 0), Some(42));
    assert_eq!(coordinator.get_committed_offset("billing-cg", 1), None);

    // Member leave
    assert!(coordinator.leave_group("billing-cg", "consumer-B"));
    coordinator.rebalance("billing-cg", 6).unwrap();
    let parts_a_all = coordinator.get_member_assignments("billing-cg", "consumer-A").unwrap();
    assert_eq!(parts_a_all.len(), 6);
}

/// [FEAT: 08-092, 08-095, 08-096, 08-098, 08-010] At-Least-Once Delivery & Dead Letter Queue
#[test]
fn test_delivery_timeout_and_dead_letter_queue() {
    let delivery = DeliveryCoordinator::new(3, 5); // Max 3 attempts, 5s timeout

    let rec = Record::new(1, 100, None, b"faulty-payload".to_vec(), HashMap::new());
    delivery.dispatch(0, rec.clone(), "consumer-1", 1000);
    assert_eq!(delivery.inflight_len(), 1);

    // Normal ack test
    let ok_rec = Record::new(2, 100, None, b"good-payload".to_vec(), HashMap::new());
    delivery.dispatch(0, ok_rec, "consumer-1", 1000);
    assert!(delivery.acknowledge(0, 2));

    // First timeout at t=1006 -> Trigger redelivery attempt 1
    let redeliveries = delivery.process_timeouts(1006);
    assert_eq!(redeliveries.len(), 1);
    assert_eq!(redeliveries[0].1.offset, 1);
    assert_eq!(delivery.dlq_len(), 0);

    // Second timeout at t=1012 -> Trigger redelivery attempt 2
    let redeliveries2 = delivery.process_timeouts(1012);
    assert_eq!(redeliveries2.len(), 1);
    assert_eq!(delivery.dlq_len(), 0);

    // Third timeout at t=1018 -> Exceeds max 3 attempts -> Route to DLQ
    let redeliveries3 = delivery.process_timeouts(1018);
    assert!(redeliveries3.is_empty());
    assert_eq!(delivery.dlq_len(), 1);
    assert_eq!(delivery.inflight_len(), 0);
}

/// [FEAT: 08-105, 08-108, 08-112] Log Compaction & Tombstone Eviction
#[test]
fn test_log_compaction_and_retention() {
    let r1 = Record::new(0, 100, Some(b"user:1".to_vec()), b"v1".to_vec(), HashMap::new());
    let r2 = Record::new(1, 200, Some(b"user:2".to_vec()), b"v1".to_vec(), HashMap::new());
    let r3 = Record::new(2, 300, Some(b"user:1".to_vec()), b"v2".to_vec(), HashMap::new());
    let r4 = Record::new(3, 400, Some(b"user:2".to_vec()), b"".to_vec(), HashMap::new()); // Tombstone for user:2
    let r5 = Record::new(4, 500, None, b"unkeyed-metric".to_vec(), HashMap::new());

    let records = vec![r1, r2, r3, r4, r5];
    let compacted = Compactor::compact_records(records);

    assert_eq!(compacted.len(), 2);
    // user:1 has latest value v2 at offset 2
    assert_eq!(compacted[0].offset, 2);
    assert_eq!(compacted[0].value, b"v2".to_vec());
    // user:2 was deleted by tombstone
    // unkeyed record preserved
    assert_eq!(compacted[1].offset, 4);

    // Retention filtering
    let time_filtered = Compactor::filter_expired(compacted, 300);
    assert_eq!(time_filtered.len(), 2);
    let aggressive_filtered = Compactor::filter_expired(time_filtered, 450);
    assert_eq!(aggressive_filtered.len(), 1);
    assert_eq!(aggressive_filtered[0].offset, 4);
}
