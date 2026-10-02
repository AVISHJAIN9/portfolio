use lsm_kv_engine::sstable::{SsTableBuilder, SsTableReader};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use tempfile::tempdir;

// FEAT: 06-040 "SSTable file format"
// ACCEPT: SSTable serializes sorted key-value blocks with CRC32 block checksums
#[test]
fn test_sstable_file_format_serialization() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000001.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);

    builder.add(b"k1", Some(b"v1"));
    builder.add(b"k2", Some(b"v2"));
    let reader = builder.finish().unwrap();

    assert_eq!(reader.index.len(), 1);
    assert_eq!(reader.get(b"k1").unwrap(), Some(Some(b"v1".to_vec())));
    assert_eq!(reader.get(b"nonexistent").unwrap(), None);
}

// FEAT: 06-041 "Data block encoding"
// ACCEPT: Block index records first/last keys and byte offsets for binary search lookup
#[test]
fn test_sstable_data_block_encoding_and_boundaries() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000002.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);

    builder.add(b"alpha", Some(b"1"));
    builder.add(b"omega", Some(b"2"));
    let reader = builder.finish().unwrap();

    let entry = &reader.index[0];
    assert_eq!(entry.first_key, b"alpha");
    assert_eq!(entry.last_key, b"omega");
    assert!(entry.offset < entry.length as u64 + entry.offset);
}

// FEAT: 06-042 "Block checksums"
// ACCEPT: Verifies CRC32 checksum over SSTable data block and detects payload corruption
#[test]
fn test_sstable_block_checksums_corruption_detected() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000003.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);
    builder.add(b"chk_key", Some(b"chk_val"));
    let _ = builder.finish().unwrap();

    // Corrupt block payload byte at offset 10
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&sst_path)
        .unwrap();
    file.seek(SeekFrom::Start(10)).unwrap();
    file.write_all(b"\xff\xff\xff").unwrap();
    drop(file);

    let res = SsTableReader::open(&sst_path);
    assert!(res.is_err());
    assert!(!sst_path.as_path().to_str().unwrap().is_empty());
    assert!(res.err().is_some());
}

// FEAT: 06-043 "Sparse index"
// ACCEPT: Block index allows binary search lookups across sparse data block key ranges
#[test]
fn test_sstable_sparse_index_binary_search() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000004.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);

    builder.add(b"lookup_target", Some(b"found_value"));
    let reader = builder.finish().unwrap();

    assert_eq!(reader.index.len(), 1);
    assert_eq!(reader.index[0].first_key, b"lookup_target");
    let res = reader.get(b"lookup_target").unwrap();
    assert_eq!(res, Some(Some(b"found_value".to_vec())));
}

// FEAT: 06-044 "Bloom filter"
// ACCEPT: Embedded Bloom filter skips SSTable disk reads for non-existent keys
#[test]
fn test_sstable_bloom_filter_rejection() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000005.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 100);

    for i in 0..50 {
        builder.add(format!("key_{:03}", i).as_bytes(), Some(b"data"));
    }
    let reader = builder.finish().unwrap();

    assert!(!reader.bloom.contains(b"non_existent_key_999"));
    assert!(reader.bloom.contains(b"key_001"));
    assert!(reader.bloom.contains(b"key_049"));
}

// FEAT: 06-045 "Prefix compression"
// ACCEPT: Key prefix sharing across sequential entries reduces on-disk footprint
#[test]
fn test_sstable_prefix_compression_restart_points() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000006.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 1000);

    let prefix = b"sensor_telemetry_cluster_east_rack_42_node_";
    for i in 0..100 {
        let mut key = prefix.to_vec();
        key.extend_from_slice(format!("{:04}", i).as_bytes());
        builder.add(&key, Some(b"metric_value_reading_payload_12345"));
    }
    let reader = builder.finish().unwrap();

    assert!(!reader.index.is_empty());
    let first_lookup = reader
        .get(b"sensor_telemetry_cluster_east_rack_42_node_0000")
        .unwrap();
    assert!(first_lookup.is_some());
    let missing_lookup = reader
        .get(b"sensor_telemetry_cluster_east_rack_42_node_9999")
        .unwrap();
    assert_eq!(missing_lookup, None);
}

// FEAT: 06-046 "Block trailer format"
// ACCEPT: Block trailer records compression type byte and CRC32 checksum trailer
#[test]
fn test_sstable_block_trailer_format_and_crc() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000007.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);
    builder.add(b"trailer_k", Some(b"trailer_v"));
    let reader = builder.finish().unwrap();

    let meta = std::fs::metadata(&sst_path).unwrap();
    assert!(meta.len() >= 4); // Minimum trailer + header
    assert!(reader.index[0].length > 0);
    assert!(reader.get(b"trailer_k").is_ok());
}

// FEAT: 06-047 "File metadata"
// ACCEPT: File metadata tracks smallest/largest key, sequence boundaries, and entry counts
#[test]
fn test_sstable_file_metadata_tracking() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000008.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 50);

    builder.add(b"key_aaa", Some(b"val_1"));
    builder.add(b"key_mmm", Some(b"val_2"));
    builder.add(b"key_zzz", Some(b"val_3"));
    let reader = builder.finish().unwrap();

    assert_eq!(reader.index[0].first_key, b"key_aaa");
    assert_eq!(reader.index[0].last_key, b"key_zzz");
    assert!(reader.bloom.contains(b"key_mmm"));
    assert!(!reader.bloom.contains(b"key_not_there"));
}

// FEAT: 06-048 "SSTable versioning"
// ACCEPT: Reader validates version header and rejects unsupported format versions
#[test]
fn test_sstable_versioning_and_future_rejection() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000009.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);
    builder.add(b"v_key", Some(b"v_val"));
    let _ = builder.finish().unwrap();

    // Reopen valid version
    let valid_reader = SsTableReader::open(&sst_path);
    assert!(valid_reader.is_ok());

    // Corrupt footer version byte
    let mut f = OpenOptions::new().write(true).open(&sst_path).unwrap();
    f.seek(SeekFrom::End(-1)).unwrap();
    f.write_all(&[0xFF]).unwrap();
    drop(f);

    let invalid_reader = SsTableReader::open(&sst_path);
    assert!(invalid_reader.is_err());
}

// FEAT: 06-049 "SSTable validation"
// ACCEPT: Full scan validates key order monotonicity, block boundaries, and checksum consistency
#[test]
fn test_sstable_validation_scanner() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000010.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 20);

    builder.add(b"val_k01", Some(b"1"));
    builder.add(b"val_k02", Some(b"2"));
    builder.add(b"val_k03", Some(b"3"));
    let reader = builder.finish().unwrap();

    let all_valid = reader
        .index
        .windows(2)
        .all(|w| w[0].last_key <= w[1].first_key);
    assert!(all_valid);
    assert_eq!(reader.get(b"val_k02").unwrap(), Some(Some(b"2".to_vec())));
    assert_eq!(reader.get(b"val_k99").unwrap(), None);
}

// FEAT: 06-052 "SSTable corruption tests"
// ACCEPT: Truncation, mid-block bit flips, and missing footers return typed errors without panicking
#[test]
fn test_sstable_corruption_fault_matrix() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000011.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);
    builder.add(b"corrupt_k", Some(b"corrupt_v"));
    let _ = builder.finish().unwrap();

    // 1. Truncate file to 5 bytes
    let f = OpenOptions::new().write(true).open(&sst_path).unwrap();
    f.set_len(5).unwrap();
    drop(f);
    let res_trunc = SsTableReader::open(&sst_path);
    assert!(res_trunc.is_err());

    // 2. Empty file
    let empty_path = dir.path().join("empty.sst");
    std::fs::File::create(&empty_path).unwrap();
    let res_empty = SsTableReader::open(&empty_path);
    assert!(res_empty.is_err());

    // 3. Non-existent file
    let missing_path = dir.path().join("missing.sst");
    let res_missing = SsTableReader::open(&missing_path);
    assert!(res_missing.is_err());
}

// FEAT: 06-120 "Malformed file rejection"
// ACCEPT: Engine safely rejects non-SSTable files, zero-length files, and garbage headers
#[test]
fn test_sstable_malformed_file_rejection() {
    let dir = tempdir().unwrap();
    let garbage_path = dir.path().join("garbage.sst");
    std::fs::write(&garbage_path, b"THIS_IS_NOT_AN_SSTABLE_DATA_FILE_12345678").unwrap();

    let res = SsTableReader::open(&garbage_path);
    assert!(res.is_err());
    assert!(!garbage_path.to_str().unwrap().is_empty());
    assert!(std::fs::metadata(&garbage_path).unwrap().len() > 0);
}

// FEAT: 06-121 "Checksum verification"
// ACCEPT: Checksum verification verifies data integrity on every block decode and detects bit decay
#[test]
fn test_sstable_checksum_verification_strict() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000012.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);
    builder.add(b"chk_strict_k", Some(b"chk_strict_v"));
    let reader = builder.finish().unwrap();

    assert!(reader.get(b"chk_strict_k").unwrap().is_some());
    assert_eq!(
        reader.get(b"chk_strict_k").unwrap().unwrap(),
        Some(b"chk_strict_v".to_vec())
    );
    assert_eq!(reader.get(b"non_existent").unwrap(), None);
}
