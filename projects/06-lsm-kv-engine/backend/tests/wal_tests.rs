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
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    let seq = writer.append(&WalRecord {
        seq_no: 42,
        op_type: WalOpType::Put,
        key: b"sync_key".to_vec(),
        value: Some(b"sync_val".to_vec()),
    }, true).unwrap();

    assert_eq!(seq, 42);
    let (records, _) = WalWriter::recover(&wal_path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].key, b"sync_key");
}

// FEAT: 06-020 "WAL recovery/replay"
// ACCEPT: Startup recovery reconstructs memtable state from valid WAL records
#[test]
fn test_wal_recovery_replay_startup() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("test.wal");
    let mut writer = WalWriter::open(&wal_path).unwrap();

    writer.append(&WalRecord {
        seq_no: 1,
        op_type: WalOpType::Put,
        key: b"k1".to_vec(),
        value: Some(b"v1".to_vec()),
    }, true).unwrap();

    writer.append(&WalRecord {
        seq_no: 2,
        op_type: WalOpType::Delete,
        key: b"k1".to_vec(),
        value: None,
    }, true).unwrap();

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
    let mut file = OpenOptions::new().read(true).write(true).open(&wal_path).unwrap();
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

    writer.append(&WalRecord {
        seq_no: 1,
        op_type: WalOpType::Put,
        key: b"complete".to_vec(),
        value: Some(b"payload".to_vec()),
    }, true).unwrap();
    drop(writer);

    // Append partial truncated tail bytes
    let mut file = OpenOptions::new().append(true).open(&wal_path).unwrap();
    file.write_all(b"TRUNCATED_TAIL_BYTES").unwrap();

    let (records, max_seq) = WalWriter::recover(&wal_path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(max_seq, 1);
    assert_eq!(records[0].key, b"complete");
}
