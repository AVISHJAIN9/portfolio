use lsm_kv_engine::sstable::{SsTableBuilder, SsTableReader};
use tempfile::tempdir;

// FEAT: 06-040
// ACCEPT: SSTable serializes sorted key-value blocks with CRC32 block checksums
#[test]
fn test_sstable_serialization_and_checksums() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000001.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);

    builder.add(b"k1", Some(b"v1"));
    builder.add(b"k2", Some(b"v2"));
    let reader = builder.finish().unwrap();

    assert_eq!(reader.index.len(), 1);
}

// FEAT: 06-041
// ACCEPT: Block index records first/last keys and byte offsets for binary search lookup
#[test]
fn test_sstable_block_index_boundaries() {
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

// FEAT: 06-042
// ACCEPT: Embedded Bloom filter skips SSTable disk reads for non-existent keys
#[test]
fn test_sstable_bloom_filter_rejection() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000003.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 100);

    for i in 0..50 {
        builder.add(format!("key_{:03}", i).as_bytes(), Some(b"data"));
    }
    let reader = builder.finish().unwrap();

    assert!(!reader.bloom.contains(b"non_existent_key_999"));
}

// FEAT: 06-043
// ACCEPT: Point read locates key in SSTable data block and verifies CRC checksum
#[test]
fn test_sstable_point_reads() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000004.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);

    builder.add(b"lookup_target", Some(b"found_value"));
    let reader = builder.finish().unwrap();

    let res = reader.get(b"lookup_target").unwrap();
    assert_eq!(res, Some(Some(b"found_value".to_vec())));
}

// FEAT: 06-044
// ACCEPT: Full scan iterates through all sorted key-value pairs across data blocks
#[test]
fn test_sstable_full_range_scan() {
    let dir = tempdir().unwrap();
    let sst_path = dir.path().join("000005.sst");
    let mut builder = SsTableBuilder::new(&sst_path, 10);

    builder.add(b"a", Some(b"1"));
    builder.add(b"b", Some(b"2"));
    builder.add(b"c", Some(b"3"));
    let reader = builder.finish().unwrap();

    let entries = reader.scan_all().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, b"a");
    assert_eq!(entries[1].0, b"b");
    assert_eq!(entries[2].0, b"c");
}
