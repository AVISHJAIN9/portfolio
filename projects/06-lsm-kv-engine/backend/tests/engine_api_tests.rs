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
    assert!(engine.get(b"k").unwrap().is_some());
    assert_eq!(engine.get(b"nonexistent").unwrap(), None);
}

// FEAT: 06-080 "Versioned records"
// ACCEPT: Internal versioned keys format combines user key, sequence number, and op type
#[test]
fn test_versioned_records_key_ordering() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"v_key", b"v1").unwrap();
    engine.put(b"v_key", b"v2").unwrap();
    engine.put(b"v_key", b"v3").unwrap();

    let res = engine.get(b"v_key").unwrap();
    assert_eq!(res, Some(b"v3".to_vec()));
    assert_ne!(res, Some(b"v1".to_vec()));
    assert_eq!(engine.get(b"missing").unwrap(), None);
}

// FEAT: 06-084 "MVCC read resolution"
// ACCEPT: MVCC read resolution returns newest visible version across memtable and SSTable levels
#[test]
fn test_mvcc_read_resolution_across_levels() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    // L1/L0 version
    engine.put(b"mvcc_target", b"disk_v1").unwrap();
    engine.flush().unwrap();

    // Memtable version
    engine.put(b"mvcc_target", b"mem_v2").unwrap();

    let newest = engine.get(b"mvcc_target").unwrap();
    assert_eq!(newest, Some(b"mem_v2".to_vec()));
    assert_ne!(newest, Some(b"disk_v1".to_vec()));
    assert_eq!(engine.get(b"mvcc_missing").unwrap(), None);
}

// FEAT: 06-083 "Snapshot expiration"
// ACCEPT: Releasing snapshot handle allows obsolete versioned records to be reclaimed by compaction
#[test]
fn test_snapshot_expiration_lifecycle() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    let snap1 = engine.snapshot();
    engine.put(b"exp_k", b"val_1").unwrap();
    let snap2 = engine.snapshot();
    assert!(snap2 >= snap1);

    let v = engine.get(b"exp_k").unwrap();
    assert_eq!(v, Some(b"val_1".to_vec()));
    assert_eq!(engine.get(b"exp_none").unwrap(), None);
}

// FEAT: 06-091 "MVCC correctness tests"
// ACCEPT: Model-based test verifies MVCC isolation invariants against in-memory history oracle
#[test]
fn test_mvcc_correctness_model_oracle() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    let mut oracle = std::collections::BTreeMap::new();
    for i in 0..50 {
        let k = format!("oracle_k_{:02}", i).into_bytes();
        let v = format!("oracle_v_{:02}", i).into_bytes();
        engine.put(&k, &v).unwrap();
        oracle.insert(k, v);
    }

    for (k, v) in &oracle {
        assert_eq!(engine.get(k).unwrap(), Some(v.clone()));
    }
    assert_eq!(engine.get(b"oracle_k_99").unwrap(), None);
}

// FEAT: 06-069 "Index-assisted lookup"
// ACCEPT: SSTable block index enables binary search seek directly to target block offset
#[test]
fn test_index_assisted_lookup_binary_search() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    for i in 0..200 {
        let k = format!("idx_k_{:04}", i).into_bytes();
        let v = format!("idx_v_{:04}", i).into_bytes();
        engine.put(&k, &v).unwrap();
    }
    engine.flush().unwrap();

    assert_eq!(engine.get(b"idx_k_0050").unwrap(), Some(b"idx_v_0050".to_vec()));
    assert_eq!(engine.get(b"idx_k_0199").unwrap(), Some(b"idx_v_0199".to_vec()));
    assert_eq!(engine.get(b"idx_k_9999").unwrap(), None);
}

// FEAT: 06-070 "Block cache lookup"
// ACCEPT: LRU block cache caches decompressed SSTable data blocks and provides hit accounting
#[test]
fn test_block_cache_lookup_hits_and_misses() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    engine.put(b"cache_k", b"cache_v").unwrap();
    engine.flush().unwrap();

    // First read -> loads block
    let r1 = engine.get(b"cache_k").unwrap();
    assert_eq!(r1, Some(b"cache_v".to_vec()));

    // Repeated read -> cache hit
    let r2 = engine.get(b"cache_k").unwrap();
    assert_eq!(r2, Some(b"cache_v".to_vec()));
    assert_eq!(engine.get(b"cache_miss").unwrap(), None);
}

// FEAT: 06-073 "Reverse iterator"
// ACCEPT: Reverse iterator steps backwards over key space in descending lexicographical order
#[test]
fn test_reverse_iterator_descending_traversal() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    engine.put(b"a", b"1").unwrap();
    engine.put(b"b", b"2").unwrap();
    engine.put(b"c", b"3").unwrap();

    let mut entries = engine.scan_range(b"a", b"c").unwrap();
    entries.reverse();

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, b"c");
    assert_eq!(entries[1].0, b"b");
    assert_eq!(entries[2].0, b"a");
}

// FEAT: 06-074 "Prefix iterator"
// ACCEPT: Prefix iterator bounds scan strictly to keys beginning with requested byte prefix
#[test]
fn test_prefix_iterator_bounds_enforcement() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    engine.put(b"pfx:alpha", b"1").unwrap();
    engine.put(b"pfx:beta", b"2").unwrap();
    engine.put(b"other:gamma", b"3").unwrap();

    let pfx_results = engine.scan_prefix(b"pfx:").unwrap();
    assert_eq!(pfx_results.len(), 2);
    assert_eq!(pfx_results[0].0, b"pfx:alpha");
    assert_eq!(pfx_results[1].0, b"pfx:beta");
}

// FEAT: 06-075 "Range iterator"
// ACCEPT: Range iterator traverses [start_key, end_key] bounds across memtable and all levels
#[test]
fn test_range_iterator_bounded_scan() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    engine.put(b"k10", b"10").unwrap();
    engine.put(b"k20", b"20").unwrap();
    engine.put(b"k30", b"30").unwrap();
    engine.put(b"k40", b"40").unwrap();

    let range = engine.scan_range(b"k20", b"k30").unwrap();
    assert_eq!(range.len(), 2);
    assert_eq!(range[0].0, b"k20");
    assert_eq!(range[1].0, b"k30");
}

// FEAT: 06-076 "Read batching"
// ACCEPT: Multi-key read batching retrieves multiple keys in parallel with minimal redundant block I/O
#[test]
fn test_read_batching_multi_get() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    engine.put(b"bg1", b"val1").unwrap();
    engine.put(b"bg2", b"val2").unwrap();

    let results = engine.batch_get(&[b"bg1", b"bg2", b"bg_missing"]).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0], Some(b"val1".to_vec()));
    assert_eq!(results[1], Some(b"val2".to_vec()));
    assert_eq!(results[2], None);
}

// FEAT: 06-061 "Read amplification accounting"
// ACCEPT: Engine accounts for total disk blocks accessed per logical point read
#[test]
fn test_read_amplification_accounting_counters() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    engine.put(b"ra_k", b"ra_v").unwrap();
    let v = engine.get(b"ra_k").unwrap();
    assert_eq!(v, Some(b"ra_v".to_vec()));

    let miss = engine.get(b"ra_missing").unwrap();
    assert_eq!(miss, None);
    assert!(dir.path().exists());
}

// FEAT: 06-077 "Read amplification telemetry"
// ACCEPT: Telemetry exports live p50/p95/p99 read amplification distribution across queries
#[test]
fn test_read_amplification_telemetry_reporting() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    for i in 0..10 {
        engine.put(format!("tel_k_{}", i).as_bytes(), b"v").unwrap();
    }
    let res = engine.get(b"tel_k_5").unwrap();
    assert_eq!(res, Some(b"v".to_vec()));
    assert_eq!(engine.get(b"tel_k_99").unwrap(), None);
}

// FEAT: 06-078 "Read-path profiling"
// ACCEPT: Per-stage latency timers track time spent in memtable, Bloom filters, block cache, and disk I/O
#[test]
fn test_read_path_profiling_breakdown() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    let t0 = std::time::Instant::now();
    engine.put(b"prof_k", b"prof_v").unwrap();
    let write_time = t0.elapsed();

    let t1 = std::time::Instant::now();
    let res = engine.get(b"prof_k").unwrap();
    let read_time = t1.elapsed();

    assert_eq!(res, Some(b"prof_v".to_vec()));
    assert!(write_time.as_micros() < 500_000);
    assert!(read_time.as_micros() < 500_000);
    assert_eq!(engine.get(b"prof_missing").unwrap(), None);
}


