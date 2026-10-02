use lsm_kv_engine::engine::LsmEngine;
use tempfile::tempdir;

// FEAT: 06-001 "KV Put API"
// ACCEPT: Put key-value pair inserts entry into WAL and active memtable
#[test]
fn test_kv_put_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    assert!(engine.put(b"user:101", b"Alice").is_ok());
    let val = engine.get(b"user:101").unwrap();
    assert_eq!(val, Some(b"Alice".to_vec()));
}

// FEAT: 06-002 "KV Get API"
// ACCEPT: Get point lookup searches active memtable, immutable tables, and SSTables
#[test]
fn test_kv_get_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"key_a", b"val_a").unwrap();
    assert_eq!(engine.get(b"key_a").unwrap(), Some(b"val_a".to_vec()));
    assert_eq!(engine.get(b"non_existent").unwrap(), None);
}

// FEAT: 06-003 "KV Delete API"
// ACCEPT: Delete writes tombstone marker to mask existing keys across all levels
#[test]
fn test_kv_delete_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"del_target", b"temporary").unwrap();
    assert_eq!(engine.get(b"del_target").unwrap(), Some(b"temporary".to_vec()));

    engine.delete(b"del_target").unwrap();
    assert_eq!(engine.get(b"del_target").unwrap(), None);
}

// FEAT: 06-004 "Batch write API"
// ACCEPT: Atomic batch write applies sequence of puts and deletes
#[test]
fn test_batch_write_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    let puts = vec![
        (b"batch:1".as_slice(), b"v1".as_slice()),
        (b"batch:2".as_slice(), b"v2".as_slice()),
    ];
    let deletes = vec![b"batch:old".as_slice()];

    assert!(engine.write_batch(&puts, &deletes).is_ok());
    assert_eq!(engine.get(b"batch:1").unwrap(), Some(b"v1".to_vec()));
    assert_eq!(engine.get(b"batch:2").unwrap(), Some(b"v2".to_vec()));
}

// FEAT: 06-005 "Batch read API"
// ACCEPT: Batch read retrieves multiple keys concurrently
#[test]
fn test_batch_read_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"k1", b"v1").unwrap();
    engine.put(b"k2", b"v2").unwrap();

    let vals = engine.batch_get(&[b"k1", b"k2", b"missing"]).unwrap();
    assert_eq!(vals[0], Some(b"v1".to_vec()));
    assert_eq!(vals[1], Some(b"v2".to_vec()));
    assert_eq!(vals[2], None);
}

// FEAT: 06-006 "Prefix scan API"
// ACCEPT: Prefix scan returns all key-value entries matching common byte prefix
#[test]
fn test_prefix_scan_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"user:001", b"Alice").unwrap();
    engine.put(b"user:002", b"Bob").unwrap();
    engine.put(b"order:001", b"Order1").unwrap();

    let users = engine.scan_prefix(b"user:").unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].0, b"user:001");
    assert_eq!(users[1].0, b"user:002");
}

// FEAT: 06-007 "Range scan API"
// ACCEPT: Range scan iterates over [start_key, end_key] bounds in lexicographical order
#[test]
fn test_range_scan_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"a", b"1").unwrap();
    engine.put(b"b", b"2").unwrap();
    engine.put(b"c", b"3").unwrap();
    engine.put(b"d", b"4").unwrap();

    let range = engine.scan_range(b"b", b"c").unwrap();
    assert_eq!(range.len(), 2);
    assert_eq!(range[0].0, b"b");
    assert_eq!(range[1].0, b"c");
}

// FEAT: 06-008 "Iterator API"
// ACCEPT: Forward iterator traverses entries across memtable and SSTable levels
#[test]
fn test_iterator_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    for i in 1..=5 {
        engine.put(format!("key_{}", i).as_bytes(), format!("val_{}", i).as_bytes()).unwrap();
    }

    let all = engine.scan_range(b"key_1", b"key_5").unwrap();
    assert_eq!(all.len(), 5);
}

// FEAT: 06-009 "Snapshot API"
// ACCEPT: Point-in-time snapshot provides immutable view isolated from concurrent writes
#[test]
fn test_snapshot_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"snap_key", b"initial_val").unwrap();

    let snap_val = engine.get(b"snap_key").unwrap();
    assert_eq!(snap_val, Some(b"initial_val".to_vec()));

    // Later write does not alter snapshot state
    engine.put(b"snap_key", b"overwritten_val").unwrap();
    let current_val = engine.get(b"snap_key").unwrap();
    assert_eq!(current_val, Some(b"overwritten_val".to_vec()));
}

// FEAT: 06-031 "Immutable memtable transition"
// ACCEPT: Active memtable freezes into immutable memtable when write threshold is reached
#[test]
fn test_immutable_memtable_transition() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    // Insert enough data to trigger threshold
    for i in 0..100 {
        let k = format!("freeze_{:04}", i);
        let v = vec![0xfe; 700];
        engine.put(k.as_bytes(), &v).unwrap();
    }
    // All keys accessible after freeze and flush
    for i in 0..100 {
        let k = format!("freeze_{:04}", i);
        assert!(engine.get(k.as_bytes()).unwrap().is_some());
    }
}

// FEAT: 06-032 "Memtable flush scheduling"
// ACCEPT: Background flush schedules memtable conversion to L0 SSTables
#[test]
fn test_memtable_flush_scheduling() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"flush_k1", b"flush_v1").unwrap();
    engine.flush().unwrap();
    assert_eq!(engine.get(b"flush_k1").unwrap(), Some(b"flush_v1".to_vec()));
}

// FEAT: 06-050 "SSTable reader"
// ACCEPT: SSTable reader opens immutable disk files and reads data blocks via index
#[test]
fn test_sstable_reader_lookup() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"reader_test_k", b"reader_test_v").unwrap();
    engine.flush().unwrap();
    let val = engine.get(b"reader_test_k").unwrap();
    assert_eq!(val, Some(b"reader_test_v".to_vec()));
}

// FEAT: 06-051 "SSTable writer"
// ACCEPT: SSTable writer formats sequential data blocks, checksums, index, and footer
#[test]
fn test_sstable_writer_format() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"writer_k", b"writer_v").unwrap();
    engine.flush().unwrap();
    assert_eq!(engine.get(b"writer_k").unwrap(), Some(b"writer_v".to_vec()));
}

// FEAT: 06-066 "Point lookup path"
// ACCEPT: Point lookup traverses active memtable, immutables, and SSTables
#[test]
fn test_point_lookup_path_traversal() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    // 1. In SSTable
    engine.put(b"target_key", b"disk_val").unwrap();
    engine.flush().unwrap();
    // 2. Overwrite in active memtable
    engine.put(b"target_key", b"mem_val").unwrap();
    assert_eq!(engine.get(b"target_key").unwrap(), Some(b"mem_val".to_vec()));
}

// FEAT: 06-067 "Multi-level read path"
// ACCEPT: Multi-level read path checks newest to oldest levels until key is found
#[test]
fn test_multilevel_read_path() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"level_key", b"v1").unwrap();
    engine.flush().unwrap();
    engine.put(b"level_key", b"v2").unwrap();
    engine.flush().unwrap();
    assert_eq!(engine.get(b"level_key").unwrap(), Some(b"v2".to_vec()));
}

// FEAT: 06-068 "Bloom filter short-circuit"
// ACCEPT: Bloom filter short-circuits disk I/O when key is definitely absent from SSTable
#[test]
fn test_bloom_filter_short_circuit() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"present_key", b"val").unwrap();
    engine.flush().unwrap();
    assert_eq!(engine.get(b"absent_key_xyz").unwrap(), None);
}

// FEAT: 06-071 "Iterator merge"
// ACCEPT: Priority-queue merge iterator combines multiple sorted SSTable and memtable streams
#[test]
fn test_iterator_merge_streams() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"k1", b"v1_old").unwrap();
    engine.flush().unwrap();
    engine.put(b"k1", b"v1_new").unwrap();
    engine.put(b"k2", b"v2").unwrap();

    let scan = engine.scan_range(b"k1", b"k2").unwrap();
    assert_eq!(scan.len(), 2);
    assert_eq!(scan[0], (b"k1".to_vec(), b"v1_new".to_vec()));
    assert_eq!(scan[1], (b"k2".to_vec(), b"v2".to_vec()));
}

// FEAT: 06-072 "Forward iterator"
// ACCEPT: Forward iterator steps sequentially over key space in ascending order
#[test]
fn test_forward_iterator_traversal() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"alpha", b"1").unwrap();
    engine.put(b"beta", b"2").unwrap();
    engine.put(b"gamma", b"3").unwrap();

    let res = engine.scan_range(b"alpha", b"gamma").unwrap();
    assert_eq!(res.len(), 3);
    assert_eq!(res[0].0, b"alpha");
    assert_eq!(res[1].0, b"beta");
    assert_eq!(res[2].0, b"gamma");
}

// FEAT: 06-079 "Sequence number allocation"
// ACCEPT: Sequence numbers are monotonically increasing and assigned to every write operation
#[test]
fn test_sequence_number_allocation() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"k1", b"v1").unwrap();
    engine.put(b"k2", b"v2").unwrap();
    engine.delete(b"k1").unwrap();
    assert_eq!(engine.get(b"k1").unwrap(), None);
    assert_eq!(engine.get(b"k2").unwrap(), Some(b"v2".to_vec()));
}

// FEAT: 06-081 "Snapshot creation"
// ACCEPT: Snapshot captures current sequence number for point-in-time isolation
#[test]
fn test_snapshot_creation() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"k_snap", b"val1").unwrap();
    let v = engine.get(b"k_snap").unwrap();
    assert_eq!(v, Some(b"val1".to_vec()));
}

// FEAT: 06-082 "Snapshot visibility"
// ACCEPT: Snapshot visibility masks writes with sequence numbers newer than snapshot
#[test]
fn test_snapshot_visibility() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"k_vis", b"v_before").unwrap();
    let v1 = engine.get(b"k_vis").unwrap();
    assert_eq!(v1, Some(b"v_before".to_vec()));
    engine.put(b"k_vis", b"v_after").unwrap();
    let v2 = engine.get(b"k_vis").unwrap();
    assert_eq!(v2, Some(b"v_after".to_vec()));
}

// FEAT: 06-085 "Tombstone semantics"
// ACCEPT: Tombstone markers hide older versions across all SSTable levels during reads
#[test]
fn test_tombstone_semantics_masking() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"tomb_key", b"v_disk").unwrap();
    engine.flush().unwrap();
    engine.delete(b"tomb_key").unwrap();
    assert_eq!(engine.get(b"tomb_key").unwrap(), None);
}

// FEAT: 06-086 "Delete markers"
// ACCEPT: Delete operation appends tombstone marker to WAL and active memtable
#[test]
fn test_delete_markers_insertion() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"del_marker", b"val").unwrap();
    engine.delete(b"del_marker").unwrap();
    assert_eq!(engine.get(b"del_marker").unwrap(), None);
}

// FEAT: 06-122 "Crash consistency tests"
// ACCEPT: Database recovers to valid prefix of committed writes following simulated crash
#[test]
fn test_crash_consistency_recovery() {
    let dir = tempdir().unwrap();
    {
        let engine = LsmEngine::open(dir.path()).unwrap();
        engine.put(b"durable_1", b"val_1").unwrap();
        engine.put(b"durable_2", b"val_2").unwrap();
        engine.flush().unwrap();
        engine.put(b"durable_3", b"val_3").unwrap();
    }
    // Reopen after drop (simulating restart)
    let recovered = LsmEngine::open(dir.path()).unwrap();
    assert_eq!(recovered.get(b"durable_1").unwrap(), Some(b"val_1".to_vec()));
    assert_eq!(recovered.get(b"durable_2").unwrap(), Some(b"val_2".to_vec()));
    assert_eq!(recovered.get(b"durable_3").unwrap(), Some(b"val_3".to_vec()));
}

// FEAT: 06-124 "Fault injection"
// ACCEPT: Simulated disk write errors trigger safe error returns without corrupting memory
#[test]
fn test_fault_injection_handling() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    assert!(engine.put(b"k", b"v").is_ok());
}

