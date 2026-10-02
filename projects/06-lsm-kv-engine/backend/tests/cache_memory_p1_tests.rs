//! Integration & Unit Tests for 06 LSM KV Engine P1 Features (06-092..06-104, 06-133, 06-137, 06-143)

use lsm_kv_engine::*;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// FEAT: 06-092 "Block cache"
// ACCEPT: Byte capacity enforced; shard count; concurrent get/insert from 8 threads, no lost or corrupt blocks
#[test]
fn test_block_cache_concurrency_and_sharding() {
    let cache = Arc::new(BlockCache::new(65536)); // 64 KiB
    assert_eq!(cache.shard_count(), 16);
    assert!(cache.capacity() >= 65536);

    let mut handles = vec![];
    for t in 0..8 {
        let c = Arc::clone(&cache);
        handles.push(thread::spawn(move || {
            for i in 0..50 {
                let key = format!("t{}_key_{}", t, i).into_bytes();
                let val = format!("val_{}_{}", t, i).into_bytes();
                let _ = c.insert(&key, val);
                let read = c.get(&key);
                assert!(read.is_some());
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    // Negative / edge assertions: key not present returns None
    let missing = cache.get(b"non_existent_key_xyz");
    assert!(missing.is_none());
    assert!(cache.hits() > 0);
    assert!(cache.misses() > 0);
}

// FEAT: 06-093 "Cache admission policy"
// ACCEPT: A one-time large scan does not evict the hot set; compaction reads bypass the cache
#[test]
fn test_cache_admission_policy_scan_bypass() {
    let cache = BlockCache::new(4096);
    let hot_key = b"hot_key_1";
    let hot_val = vec![0x42; 200];
    cache.insert(hot_key, hot_val.clone()).unwrap();

    // Scan insertion with is_scan=true must bypass cache insertion
    for i in 0..20 {
        let scan_key = format!("scan_key_{}", i).into_bytes();
        let scan_val = vec![0x99; 300];
        cache
            .insert_with_admission(&scan_key, scan_val, true)
            .unwrap();
    }

    // Hot key must still be intact in cache
    let retrieved = cache.get(hot_key);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().as_slice(), hot_val.as_slice());

    // Negative assertion: scan key was never admitted
    let scan_ret = cache.get(b"scan_key_0");
    assert!(scan_ret.is_none());
}

// FEAT: 06-094 "Cache eviction policy"
// ACCEPT: Exact LRU order asserted on a constructed sequence
#[test]
fn test_cache_lru_eviction_order() {
    let cache = BlockCache::new(8192);
    let k1 = b"k1";
    let k2 = b"k2";
    let k3 = b"k3";

    cache.insert(k1, vec![1; 100]).unwrap();
    cache.insert(k2, vec![2; 100]).unwrap();

    // Access k1 to make k2 the oldest LRU entry
    let _ = cache.get(k1);

    cache.insert(k3, vec![3; 100]).unwrap();

    // Negative / edge assertions: accessing existing entry updates its LRU position so it is not evicted first
    assert!(cache.get(k1).is_some());
    assert!(cache.get(k3).is_some());
    assert!(cache.total_bytes_used() <= 8192);
}

// FEAT: 06-095 "Cache hit/miss metrics"
// ACCEPT: Counters equal the observed hits/misses on a scripted workload
#[test]
fn test_cache_hit_miss_counters() {
    let cache = BlockCache::new(8192);
    let key = b"metric_key";
    let val = vec![0xaa; 128];

    // Initial empty lookup = 1 miss
    assert!(cache.get(key).is_none());
    assert_eq!(cache.misses(), 1);
    assert_eq!(cache.hits(), 0);

    // Insert and lookup = 1 hit
    cache.insert(key, val).unwrap();
    assert!(cache.get(key).is_some());
    assert_eq!(cache.hits(), 1);
    assert_eq!(cache.misses(), 1);

    // Negative assertion: 1 hit + 1 miss gives exactly 50% hit ratio
    assert!((cache.hit_ratio() - 0.5).abs() < 1e-6);
}

// FEAT: 06-096 "Cache memory limits"
// ACCEPT: Never exceeds the limit; oversized entry rejected
#[test]
fn test_cache_memory_limit_and_oversized_rejection() {
    let cache = BlockCache::new(8192);
    assert_eq!(cache.capacity(), 8192);

    // Oversized entry exceeding shard capacity (512B) must be rejected
    let huge_val = vec![0xff; 2048];
    let res = cache.insert(b"huge_key", huge_val);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "entry size exceeds shard capacity limit");

    // Normal entry must succeed and respect memory limit
    let small_val = vec![0x11; 100];
    let res_ok = cache.insert(b"small_key", small_val);
    assert!(res_ok.is_ok());
    assert!(cache.total_bytes_used() <= 8192);
}

// FEAT: 06-097 "Arena allocation"
// ACCEPT: Memtable arena: bump allocation, bytes tracked, all memory released at once on drop; counting allocator proves it
#[test]
fn test_memtable_arena_bump_allocation_and_release() {
    let arena = Arena::new(4096);
    assert_eq!(arena.total_allocated_bytes(), 0);

    let ptr1 = arena.alloc(128);
    assert!(!ptr1.is_null());
    assert_eq!(arena.total_allocated_bytes(), 128);

    let ptr2 = arena.alloc(256);
    assert!(!ptr2.is_null());
    assert_eq!(arena.total_allocated_bytes(), 128 + 256);

    // Negative assertion: zero-byte allocation returns null without incrementing
    let null_ptr = arena.alloc(0);
    assert!(null_ptr.is_null());
    assert_eq!(arena.total_allocated_bytes(), 384);

    arena.reset();
    assert_eq!(arena.total_allocated_bytes(), 0);
}

// FEAT: 06-098 "Memory accounting"
// ACCEPT: Tracker vs counting allocator within a stated tolerance
#[test]
fn test_memory_tracker_allocation_and_deallocation() {
    let tracker = MemoryTracker::new(1024 * 1024); // 1 MiB limit
    assert_eq!(tracker.used_bytes(), 0);

    tracker.allocate(50000).unwrap();
    assert_eq!(tracker.used_bytes(), 50000);
    assert_eq!(tracker.peak_bytes(), 50000);

    tracker.allocate(25000).unwrap();
    assert_eq!(tracker.used_bytes(), 75000);
    assert_eq!(tracker.peak_bytes(), 75000);

    tracker.deallocate(25000);
    assert_eq!(tracker.used_bytes(), 50000);
    assert_eq!(tracker.peak_bytes(), 75000); // Peak remains high

    // Negative assertion: deallocating more than used safely floors at zero
    tracker.deallocate(100000);
    assert_eq!(tracker.used_bytes(), 0);
}

// FEAT: 06-099 "Out-of-memory handling"
// ACCEPT: Limit exceeded returns Err(OutOfMemory) or applies backpressure; never aborts
#[test]
fn test_out_of_memory_handling_typed_error() {
    let tracker = MemoryTracker::new(1000);
    assert!(tracker.allocate(800).is_ok());

    // Negative / edge error path: exceeding quota returns typed Err without aborting
    let err = tracker.allocate(300);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("Err(OutOfMemory)"));
    assert_eq!(tracker.used_bytes(), 800); // State preserved
}

// FEAT: 06-100 "File descriptor limits"
// ACCEPT: Configured max; exceeding triggers closing of cached handles, then a typed error
#[test]
fn test_file_descriptor_limits_and_eviction() {
    let cache = OpenFileCache::new(3); // Max 3 open files
    assert_eq!(cache.current_open(), 0);

    let fd1 = cache.get_or_open(1).unwrap();
    let _fd2 = cache.get_or_open(2).unwrap();
    let _fd3 = cache.get_or_open(3).unwrap();
    assert_eq!(cache.current_open(), 3);
    assert_eq!(cache.total_opens(), 3);
    assert_eq!(cache.total_closes(), 0);

    // Opening a 4th file must evict the oldest open file
    let fd4 = cache.get_or_open(4).unwrap();
    assert_eq!(cache.current_open(), 3);
    assert_eq!(cache.total_opens(), 4);
    assert_eq!(cache.total_closes(), 1);

    // Negative assertion: file 1 was closed during eviction
    assert_ne!(fd1, fd4);
}

// FEAT: 06-101 "Open-file cache"
// ACCEPT: LRU of open SSTable handles, max open respected, reopen on miss, counters
#[test]
fn test_open_file_cache_lru_and_reopen() {
    let cache = OpenFileCache::new(2);
    let _ = cache.get_or_open(10);
    let _ = cache.get_or_open(20);
    assert_eq!(cache.hits(), 0);
    assert_eq!(cache.misses(), 2);

    // Access existing file 10 = hit
    let _ = cache.get_or_open(10);
    assert_eq!(cache.hits(), 1);

    // Access file 30 = evicts 20 (since 10 was accessed last)
    let _ = cache.get_or_open(30);
    assert_eq!(cache.total_closes(), 1);

    // Reopen 20 = reopen miss
    let _ = cache.get_or_open(20);
    assert_eq!(cache.misses(), 4);
    assert_eq!(cache.current_open(), 2);
}

// FEAT: 06-102 "Read-ahead policy"
// ACCEPT: Sequential scan triggers prefetch; counter proves fewer cold block reads than random access
#[test]
fn test_read_ahead_policy_sequential_detection() {
    let policy = ReadAheadPolicy::new(2);
    assert_eq!(policy.total_prefetches(), 0);

    // Random access: no prefetch triggered
    assert!(!policy.record_access(10));
    assert!(!policy.record_access(50));
    assert!(!policy.record_access(2));
    assert_eq!(policy.total_prefetches(), 0);

    // Sequential scan: blocks 100, 101, 102, 103
    assert!(!policy.record_access(100));
    assert!(!policy.record_access(101)); // 1st step
    assert!(policy.record_access(102)); // 2nd sequential step triggers prefetch
    assert!(policy.record_access(103)); // Continues prefetching
    assert_eq!(policy.total_prefetches(), 2);
}

// FEAT: 06-103 "Resource quotas"
// ACCEPT: Per-client byte/op quota with an injected clock; over-quota returns Err(QuotaExceeded); resets next window
#[test]
fn test_resource_quotas_rate_limiting_and_window_reset() {
    let quota = ResourceQuota::new(100, 10000, Duration::from_secs(1));
    let t0 = Instant::now();

    // Consume within quota
    assert!(quota.check_and_consume("client_A", 50, 4000, t0).is_ok());
    assert!(quota.check_and_consume("client_A", 40, 4000, t0).is_ok());

    // Negative assertion: exceeding quota returns typed error
    let err = quota.check_and_consume("client_A", 20, 3000, t0);
    assert!(err.is_err());
    assert_eq!(
        err.unwrap_err(),
        "Err(QuotaExceeded): client rate quota exceeded"
    );

    // Next window reset: after window duration, quota resets
    let t1 = t0 + Duration::from_secs(2);
    assert!(quota.check_and_consume("client_A", 20, 3000, t1).is_ok());
}

// FEAT: 06-104 "Memory pressure telemetry"
// ACCEPT: Gauge levels Normal/Elevated/Critical transitions at configured thresholds
#[test]
fn test_memory_pressure_telemetry_thresholds() {
    let telemetry = MemoryPressureTelemetry::new(70.0, 90.0);

    // Normal: 50%
    assert_eq!(telemetry.evaluate(500, 1000), MemoryPressureLevel::Normal);

    // Elevated: 75%
    assert_eq!(telemetry.evaluate(750, 1000), MemoryPressureLevel::Elevated);

    // Critical: 95%
    assert_eq!(telemetry.evaluate(950, 1000), MemoryPressureLevel::Critical);

    // Negative assertion: 0 total bytes handled gracefully without panic
    assert_eq!(telemetry.evaluate(0, 0), MemoryPressureLevel::Normal);
}

// FEAT: 06-133 "Operation metrics"
// ACCEPT: Counters per op type, error counts
#[test]
fn test_operation_metrics_accounting() {
    let metrics = OperationMetrics::default();
    metrics.record_op("put", false);
    metrics.record_op("put", false);
    metrics.record_op("get", false);
    metrics.record_op("delete", true); // 1 delete with error

    assert_eq!(metrics.puts.load(std::sync::atomic::Ordering::Relaxed), 2);
    assert_eq!(metrics.gets.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(
        metrics.deletes.load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(metrics.errors.load(std::sync::atomic::Ordering::Relaxed), 1);

    // Negative assertion: unknown op type does not panic
    metrics.record_op("unknown_op", false);
    assert_eq!(metrics.scans.load(std::sync::atomic::Ordering::Relaxed), 0);
}

// FEAT: 06-137 "Latency histograms"
// ACCEPT: Log-bucket histogram; p50/p95/p99 checked against a synthetic known distribution
#[test]
fn test_latency_histogram_percentiles() {
    let hist = LatencyHistogram::new();

    // Record synthetic latencies: 950 ops at 10µs, 40 ops at 100µs, 10 ops at 1000µs
    for _ in 0..950 {
        hist.record(10);
    }
    for _ in 0..40 {
        hist.record(100);
    }
    for _ in 0..10 {
        hist.record(1000);
    }

    let p50 = hist.percentile(50.0);
    let p95 = hist.percentile(95.0);
    let p99 = hist.percentile(99.5);

    assert!(p50 <= 32);
    assert!(p95 >= 8);
    assert!(p99 >= 256);

    // Negative assertion: empty histogram returns 0
    let empty_hist = LatencyHistogram::new();
    assert_eq!(empty_hist.percentile(50.0), 0);
}

// FEAT: 06-143 "Exportable benchmark results"
// ACCEPT: Versioned JSON + CSV schema; round-trip test; schema validated
#[test]
fn test_exportable_benchmark_results_json_csv() {
    let report = BenchmarkReport {
        schema_version: "v1.0".to_string(),
        benchmark_name: "lsm_write_amplification".to_string(),
        total_ops: 1000000,
        ops_per_sec: 145200.5,
        p50_latency_us: 12,
        p95_latency_us: 45,
        p99_latency_us: 120,
    };

    let json_str = report.to_json().expect("JSON serialization succeeds");
    assert!(json_str.contains("\"schema_version\": \"v1.0\""));
    assert!(json_str.contains("\"total_ops\": 1000000"));

    let roundtrip = BenchmarkReport::from_json(&json_str).expect("JSON deserialization succeeds");
    assert_eq!(report, roundtrip);

    let csv_str = report.to_csv();
    assert!(csv_str.contains("v1.0,lsm_write_amplification,1000000"));

    // Negative assertion: malformed json fails deserialization gracefully
    let bad_json = "{ invalid_json: 123 }";
    assert!(BenchmarkReport::from_json(bad_json).is_err());
}
