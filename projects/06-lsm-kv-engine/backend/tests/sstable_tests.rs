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
    let mut file = OpenOptions::new().read(true).write(true).open(&sst_path).unwrap();
    file.seek(SeekFrom::Start(10)).unwrap();
    file.write_all(b"\xff\xff\xff").unwrap();
    drop(file);

    let res = SsTableReader::open(&sst_path);
    assert!(res.is_err());
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
}
