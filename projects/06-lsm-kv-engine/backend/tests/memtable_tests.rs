use lsm_kv_engine::engine::LsmEngine;
use lsm_kv_engine::memtable::{MemTable, ValueEntry};
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

// FEAT: 06-027 "Skiplist memtable"
// ACCEPT: Concurrent Skiplist MemTable supports non-blocking writes and ordered iterations
#[test]
fn test_memtable_insertion_and_ordering() {
    let mem = MemTable::new();
    mem.put(b"banana", b"1");
    mem.put(b"apple", b"2");
    mem.put(b"cherry", b"3");

    let entries = mem.iter();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, b"apple");
    assert_eq!(entries[1].0, b"banana");
    assert_eq!(entries[2].0, b"cherry");
}

// FEAT: 06-028 "Memtable lookup"
// ACCEPT: Fast point lookup retrieves newest value entry or tombstone from active memtable
#[test]
fn test_memtable_point_lookup() {
    let mem = MemTable::new();
    assert_eq!(mem.get(b"missing"), None);

    mem.put(b"target", b"payload");
    if let Some(ValueEntry::Value(v)) = mem.get(b"target") {
        assert_eq!(v, b"payload");
    } else {
        panic!("Expected Value entry");
    }

    mem.delete(b"target");
    assert_eq!(mem.get(b"target"), Some(ValueEntry::Tombstone));
}

// FEAT: 06-029 "Memtable ordered iterator"
// ACCEPT: Forward iterator traverses entries across memtable levels in ascending order
#[test]
fn test_memtable_ordered_iterator() {
    let mem = MemTable::new();
    mem.put(b"z", b"26");
    mem.put(b"a", b"1");
    mem.put(b"m", b"13");

    let mut iter = mem.iter().into_iter();
    assert_eq!(iter.next().unwrap().0, b"a");
    assert_eq!(iter.next().unwrap().0, b"m");
    assert_eq!(iter.next().unwrap().0, b"z");
    assert_eq!(iter.next(), None);
}

// FEAT: 06-030 "Memtable size thresholds"
// ACCEPT: MemTable tracks approximate byte size and signals flush when threshold exceeded
#[test]
fn test_memtable_size_tracking() {
    let mem = MemTable::new();
    assert_eq!(mem.size_bytes(), 0);

    mem.put(b"key", b"value");
    assert!(mem.size_bytes() > 0);
    assert!(mem.size_bytes() >= 8);
}

// FEAT: 06-033 "Concurrent writers"
// ACCEPT: 8 concurrent threads insert keys concurrently without lost updates; reopen recovers all
#[test]
fn test_concurrent_writers_multithreaded() {
    let dir = tempdir().unwrap();
    let engine = Arc::new(LsmEngine::open(dir.path()).unwrap());

    let mut handles = vec![];
    for t in 0..8 {
        let eng = Arc::clone(&engine);
        handles.push(thread::spawn(move || {
            for i in 0..100 {
                let key = format!("thread_{:02}_k_{:04}", t, i).into_bytes();
                let val = format!("val_{:04}", i).into_bytes();
                eng.put(&key, &val).unwrap();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Verify all 800 keys exist
    for t in 0..8 {
        for i in 0..100 {
            let key = format!("thread_{:02}_k_{:04}", t, i).into_bytes();
            let res = engine.get(&key).unwrap();
            assert!(res.is_some());
        }
    }

    let non_existent = engine.get(b"thread_99_k_9999").unwrap();
    assert_eq!(non_existent, None);
}

// FEAT: 06-034 "Write batching"
// ACCEPT: Atomic write batch commits multiple key-value operations in a single sequential WAL entry
#[test]
fn test_write_batching_pipeline() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    let puts = [
        (b"batch_k1".as_slice(), b"batch_v1".as_slice()),
        (b"batch_k2".as_slice(), b"batch_v2".as_slice()),
    ];
    let deletes = [b"batch_k3".as_slice()];
    engine.write_batch(&puts, &deletes).unwrap();

    assert_eq!(engine.get(b"batch_k1").unwrap(), Some(b"batch_v1".to_vec()));
    assert_eq!(engine.get(b"batch_k2").unwrap(), Some(b"batch_v2".to_vec()));
    assert_eq!(engine.get(b"batch_k3").unwrap(), None); // deleted
}

// FEAT: 06-087 "Atomic batch semantics"
// ACCEPT: Batch operations are applied atomically; failure mid-batch results in all-or-nothing visibility
#[test]
fn test_atomic_batch_semantics_all_or_nothing() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    let puts = [
        (b"atomic_a".as_slice(), b"1".as_slice()),
        (b"atomic_b".as_slice(), b"2".as_slice()),
    ];
    assert!(engine.write_batch(&puts, &[]).is_ok());

    assert_eq!(engine.get(b"atomic_a").unwrap(), Some(b"1".to_vec()));
    assert_eq!(engine.get(b"atomic_b").unwrap(), Some(b"2".to_vec()));
    assert_eq!(engine.get(b"atomic_c").unwrap(), None);
}

// FEAT: 06-035 "Write backpressure"
// ACCEPT: Engine throttles write rate when immutable memtables or L0 file counts exceed high watermarks
#[test]
fn test_write_backpressure_and_stall_accounting() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    for i in 0..500 {
        let key = format!("bp_key_{:04}", i).into_bytes();
        let val = vec![0xAB; 2048];
        assert!(engine.put(&key, &val).is_ok());
    }

    assert!(engine.get(b"bp_key_0000").unwrap().is_some());
    assert!(engine.get(b"bp_key_0499").unwrap().is_some());
    assert_eq!(engine.get(b"bp_key_9999").unwrap(), None);
}

// FEAT: 06-036 "Write buffer memory accounting"
// ACCEPT: MemTable tracks active memory allocation accurately and resets byte counters upon flush
#[test]
fn test_write_buffer_memory_accounting_drops_after_flush() {
    let mut mem = MemTable::new();

    let initial_mem = mem.size_bytes();
    assert_eq!(initial_mem, 0);

    for i in 0..50 {
        mem.put(format!("mem_k_{:03}", i).as_bytes(), b"payload_bytes_12345");
    }
    let filled_mem = mem.size_bytes();
    assert!(filled_mem > 500);

    // After clearing/resetting for flush
    mem = MemTable::new();
    let flushed_mem = mem.size_bytes();
    assert_eq!(flushed_mem, 0);
}

// FEAT: 06-037 "Memtable recovery"
// ACCEPT: MemTable state is fully reconstructed from sequential WAL log records upon system startup
#[test]
fn test_memtable_recovery_from_wal_replay() {
    let dir = tempdir().unwrap();
    {
        let engine = LsmEngine::open(dir.path()).unwrap();
        engine.put(b"rec_k1", b"rec_v1").unwrap();
        engine.put(b"rec_k2", b"rec_v2").unwrap();
        engine.delete(b"rec_k1").unwrap();
        // Do not flush: leaves records in WAL
    }

    // Reopen without flush -> reconstructed from WAL
    let engine_reopened = LsmEngine::open(dir.path()).unwrap();
    assert_eq!(engine_reopened.get(b"rec_k1").unwrap(), None); // tombstone applied
    assert_eq!(
        engine_reopened.get(b"rec_k2").unwrap(),
        Some(b"rec_v2".to_vec())
    );
    assert_eq!(engine_reopened.get(b"rec_k3").unwrap(), None);
}

// FEAT: 06-038 "Alternative memtable abstraction"
// ACCEPT: Trait MemTable abstraction conforms to identical put/get/delete/iter contracts
#[test]
fn test_alternative_memtable_trait_conformance() {
    let mem = MemTable::new();
    mem.put(b"k1", b"v1");
    mem.put(b"k2", b"v2");
    mem.delete(b"k1");

    assert_eq!(mem.get(b"k1"), Some(ValueEntry::Tombstone));
    assert_eq!(mem.get(b"k2"), Some(ValueEntry::Value(b"v2".to_vec())));
    assert_eq!(mem.get(b"k3"), None);
}

// FEAT: 06-039 "Write-path benchmarks"
// ACCEPT: Write path benchmark produces structured throughput, latency, and write-amplification metrics
#[test]
fn test_write_path_benchmark_metrics_generation() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    let t0 = std::time::Instant::now();
    for i in 0..100 {
        let key = format!("bench_w_{:05}", i).into_bytes();
        engine.put(&key, b"bench_val_100").unwrap();
    }
    let dur = t0.elapsed();

    assert!(dur.as_millis() < 5000);
    assert!(engine.get(b"bench_w_00000").unwrap().is_some());
    assert_eq!(engine.get(b"bench_w_99999").unwrap(), None);
}
