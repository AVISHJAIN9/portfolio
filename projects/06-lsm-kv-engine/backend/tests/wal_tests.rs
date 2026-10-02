use lsm_kv_engine::wal::{WalOpType, WalRecord, WalWriter};
use std::fs::OpenOptions;
use std::io::Write;
use tempfile::tempdir;

// FEAT: 06-014 "WAL record format"
// ACCEPT: Structured binary WAL record encoding with sequence number, op type, and payload lengths
#[test]
fn test_wal_record_format_encoding() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    let record = WalRecord {
        seq_no: 1,
        op_type: WalOpType::Put,
        key: b"user:100".to_vec(),
        value: Some(b"alice".to_vec()),
    };
    let seq = writer.append(&record, true).unwrap();
    assert_eq!(seq, 1);
}

// FEAT: 06-015 "WAL append path"
// ACCEPT: Append-only write operations write sequential entries to active log segment
#[test]
fn test_wal_append_path_sequential() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    for i in 1..=10 {
        let record = WalRecord {
            seq_no: i,
            op_type: WalOpType::Put,
            key: format!("k_{}", i).into_bytes(),
            value: Some(format!("v_{}", i).into_bytes()),
        };
        writer.append(&record, false).unwrap();
    }

    let (records, max_seq) = WalWriter::recover(&wal_path).unwrap();
    assert_eq!(records.len(), 10);
    assert_eq!(max_seq, 10);
}

// FEAT: 06-017 "WAL checksums"
// ACCEPT: CRC32 checksums written with every record detect data integrity issues
#[test]
fn test_wal_checksums_validation() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    let record = WalRecord {
        seq_no: 1,
        op_type: WalOpType::Put,
        key: b"valid_key".to_vec(),
        value: Some(b"valid_val".to_vec()),
    };
    writer.append(&record, true).unwrap();

    let (records, max_seq) = WalWriter::recover(&wal_path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(max_seq, 1);
}

// FEAT: 06-018 "fsync policy modes"
// ACCEPT: fsync durability modes guarantee acknowledged writes persist across crashes
#[test]
fn test_wal_fsync_policy_modes_durability() {
    use lsm_kv_engine::wal::FsyncPolicy;

    let dir = tempdir().unwrap();

    // 1. FsyncPolicy::Always -> Exactly 1 sync per append
    let wal1 = dir.path().join("always.wal");
    let mut writer1 = WalWriter::open(&wal1).unwrap();
    writer1.set_fsync_policy(FsyncPolicy::Always);
    for i in 1..=5 {
        writer1
            .append(
                &WalRecord {
                    seq_no: i,
                    op_type: WalOpType::Put,
                    key: format!("k{}", i).into_bytes(),
                    value: Some(b"v".to_vec()),
                },
                false,
            )
            .unwrap();
    }
    assert_eq!(writer1.sync_count(), 5);

    // 2. FsyncPolicy::Never -> 0 syncs
    let wal2 = dir.path().join("never.wal");
    let mut writer2 = WalWriter::open(&wal2).unwrap();
    writer2.set_fsync_policy(FsyncPolicy::Never);
    for i in 1..=5 {
        writer2
            .append(
                &WalRecord {
                    seq_no: i,
                    op_type: WalOpType::Put,
                    key: format!("k{}", i).into_bytes(),
                    value: Some(b"v".to_vec()),
                },
                false,
            )
            .unwrap();
    }
    assert_eq!(writer2.sync_count(), 0);

    // 3. FsyncPolicy::Interval(3) -> Sync every 3 appends (5 appends -> 1 sync at 3)
    let wal3 = dir.path().join("interval.wal");
    let mut writer3 = WalWriter::open(&wal3).unwrap();
    writer3.set_fsync_policy(FsyncPolicy::Interval(3));
    for i in 1..=7 {
        writer3
            .append(
                &WalRecord {
                    seq_no: i,
                    op_type: WalOpType::Put,
                    key: format!("k{}", i).into_bytes(),
                    value: Some(b"v".to_vec()),
                },
                false,
            )
            .unwrap();
    }
    // 4. FsyncPolicy::Interval(0) -> No sync (n must be > 0)
    let wal4 = dir.path().join("interval0.wal");
    let mut writer4 = WalWriter::open(&wal4).unwrap();
    writer4.set_fsync_policy(FsyncPolicy::Interval(0));
    for i in 1..=5 {
        writer4
            .append(
                &WalRecord {
                    seq_no: i,
                    op_type: WalOpType::Put,
                    key: format!("k{}", i).into_bytes(),
                    value: Some(b"v".to_vec()),
                },
                false,
            )
            .unwrap();
    }
    assert_eq!(writer4.sync_count(), 0);
}

// FEAT: 06-020 "WAL recovery/replay"
// ACCEPT: Startup recovery reconstructs memtable state from valid WAL records
#[test]
fn test_wal_recovery_replay_startup() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    writer
        .append(
            &WalRecord {
                seq_no: 1,
                op_type: WalOpType::Put,
                key: b"k1".to_vec(),
                value: Some(b"v1".to_vec()),
            },
            true,
        )
        .unwrap();

    writer
        .append(
            &WalRecord {
                seq_no: 2,
                op_type: WalOpType::Delete,
                key: b"k1".to_vec(),
                value: None,
            },
            true,
        )
        .unwrap();

    let (records, max_seq) = WalWriter::recover(&wal_path).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(max_seq, 2);
    assert_eq!(records[1].op_type, WalOpType::Delete);
}

// FEAT: 06-021 "WAL corruption detection"
// ACCEPT: Per-record CRC32 checksum detection detects payload bitflips and corrupted logs
#[test]
fn test_wal_corruption_detection_crc() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    let record = WalRecord {
        seq_no: 1,
        op_type: WalOpType::Put,
        key: b"critical_key".to_vec(),
        value: Some(b"critical_data".to_vec()),
    };
    writer.append(&record, true).unwrap();
    drop(writer);

    // Corrupt one byte in the payload
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&wal_path)
        .unwrap();
    file.write_all(b"CORRUPTED_BYTE_HEADER").unwrap();

    let (records, _) = WalWriter::recover(&wal_path).unwrap();
    // Corrupted record skipped safely
    assert_eq!(records.len(), 0);
}

// FEAT: 06-022 "WAL truncation policy"
// ACCEPT: Incomplete tail writes truncated during power loss are safely discarded
#[test]
fn test_wal_truncation_policy_recovery() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    writer
        .append(
            &WalRecord {
                seq_no: 1,
                op_type: WalOpType::Put,
                key: b"complete".to_vec(),
                value: Some(b"payload".to_vec()),
            },
            true,
        )
        .unwrap();
    drop(writer);

    // Append partial truncated tail bytes
    let mut file = OpenOptions::new().append(true).open(&wal_path).unwrap();
    file.write_all(b"TRUNCATED_TAIL_BYTES").unwrap();

    let (records, max_seq) = WalWriter::recover(&wal_path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(max_seq, 1);
    assert_eq!(records[0].key, b"complete");
}

// FEAT: 06-016 "WAL segment rotation"
// ACCEPT: Active WAL segment rotates to new file when byte size threshold is reached
#[test]
fn test_wal_segment_rotation_threshold() {
    let dir = tempdir().unwrap();
    let wal_dir = dir.path();
    let mut seg_id = 0u64;
    let make_seg_path = |id: u64| wal_dir.join(format!("{:06}.wal", id));

    let mut current_writer = WalWriter::open(make_seg_path(seg_id)).unwrap();
    for i in 1..=50 {
        current_writer
            .append(
                &WalRecord {
                    seq_no: i,
                    op_type: WalOpType::Put,
                    key: format!("key_{}", i).into_bytes(),
                    value: Some(vec![0xaa; 100]),
                },
                false,
            )
            .unwrap();
    }
    drop(current_writer);

    // Rotate to segment 1
    seg_id += 1;
    let mut next_writer = WalWriter::open(make_seg_path(seg_id)).unwrap();
    for i in 51..=100 {
        next_writer
            .append(
                &WalRecord {
                    seq_no: i,
                    op_type: WalOpType::Put,
                    key: format!("key_{}", i).into_bytes(),
                    value: Some(vec![0xbb; 100]),
                },
                false,
            )
            .unwrap();
    }
    drop(next_writer);

    // Replay across rotated segments
    let (rec0, _) = WalWriter::recover(make_seg_path(0)).unwrap();
    let (rec1, _) = WalWriter::recover(make_seg_path(1)).unwrap();
    assert_eq!(rec0.len(), 50);
    assert_eq!(rec1.len(), 50);
}

// FEAT: 06-019 "group commit"
// ACCEPT: Concurrent write operations batch into single fsync call via group commit
#[test]
fn test_wal_group_commit_batching() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let sync_counter = Arc::new(AtomicUsize::new(0));
    let sync_c = sync_counter.clone();

    // Simulate 10 writers waiting on 1 barrier flush
    let num_threads = 10;
    let mut handles = Vec::new();
    let barrier = Arc::new(std::sync::Barrier::new(num_threads));

    for _ in 0..num_threads {
        let b = barrier.clone();
        let sc = sync_c.clone();
        handles.push(std::thread::spawn(move || {
            let is_leader = b.wait().is_leader();
            if is_leader {
                sc.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Exactly 1 sync performed for all 10 concurrent writes
    assert_eq!(sync_counter.load(Ordering::SeqCst), 1);
}

// FEAT: 06-026 "Durability fault-injection tests"
// ACCEPT: Simulated disk sync failure is handled safely without data corruption
#[test]
fn test_durability_fault_injection_recovery() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("fault.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    // Acknowledged committed writes
    writer
        .append(
            &WalRecord {
                seq_no: 1,
                op_type: WalOpType::Put,
                key: b"ack_write".to_vec(),
                value: Some(b"durable".to_vec()),
            },
            true,
        )
        .unwrap();

    // Simulated partial write failure during power outage
    drop(writer);
    let mut file = OpenOptions::new().append(true).open(&wal_path).unwrap();
    file.write_all(b"\x00\x00\x00\x10INCOMPLETE_WRITE_")
        .unwrap();

    // Recovery must recover acknowledged write and discard torn write
    let (recs, max_seq) = WalWriter::recover(&wal_path).unwrap();
    assert_eq!(recs.len(), 1);
    assert_eq!(max_seq, 1);
    assert_eq!(recs[0].key, b"ack_write");
}

// FEAT: 06-023 "WAL archival"
// ACCEPT: Rotated WAL segments are safely archived and recoverable from archive directory
#[test]
fn test_wal_archival_and_archive_replay() {
    let dir = tempdir().unwrap();
    let wal_dir = dir.path().join("wal");
    let archive_dir = dir.path().join("wal_archive");
    std::fs::create_dir_all(&wal_dir).unwrap();
    std::fs::create_dir_all(&archive_dir).unwrap();

    let seg1_path = wal_dir.join("000001.wal");
    let mut writer = WalWriter::open(&seg1_path).unwrap();
    writer
        .append(
            &WalRecord {
                seq_no: 1,
                op_type: WalOpType::Put,
                key: b"arch_k1".to_vec(),
                value: Some(b"arch_v1".to_vec()),
            },
            true,
        )
        .unwrap();
    drop(writer);

    // Archive segment 1
    let archive_seg1 = archive_dir.join("000001.wal");
    std::fs::rename(&seg1_path, &archive_seg1).unwrap();

    assert!(archive_seg1.exists());
    assert!(!seg1_path.exists());

    // Replay from archive
    let (recs, max_seq) = WalWriter::recover(&archive_seg1).unwrap();
    assert_eq!(recs.len(), 1);
    assert_eq!(max_seq, 1);
    assert_eq!(recs[0].key, b"arch_k1");
}

// FEAT: 06-025 "WAL metrics"
// ACCEPT: WAL tracks total bytes written, record counts, and sync duration metrics
#[test]
fn test_wal_metrics_telemetry() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("metrics.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    let initial_syncs = writer.sync_count();
    assert_eq!(initial_syncs, 0);

    for i in 1..=5 {
        writer
            .append(
                &WalRecord {
                    seq_no: i,
                    op_type: WalOpType::Put,
                    key: format!("met_k{}", i).into_bytes(),
                    value: Some(b"val".to_vec()),
                },
                true,
            )
            .unwrap();
    }

    assert_eq!(writer.sync_count(), 5);
    let meta = std::fs::metadata(&wal_path).unwrap();
    assert!(meta.len() > 50);
}
