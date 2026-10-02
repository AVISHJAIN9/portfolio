//! Memory Management, Resource Limits, Quotas, and Telemetry (06-097..06-104, 06-133, 06-137, 06-143)

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// 06-097: Arena Allocator with bump allocation and batch deallocation on drop
pub struct Arena {
    chunks: Mutex<Vec<Vec<u8>>>,
    chunk_size: usize,
    allocated_bytes: AtomicUsize,
}

impl Arena {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunks: Mutex::new(Vec::new()),
            chunk_size: chunk_size.max(4096),
            allocated_bytes: AtomicUsize::new(0),
        }
    }

    pub fn alloc(&self, size: usize) -> *mut u8 {
        if size == 0 {
            return std::ptr::null_mut();
        }
        let mut chunks = self.chunks.lock();
        if chunks.is_empty() || chunks.last().unwrap().capacity() - chunks.last().unwrap().len() < size {
            let cap = self.chunk_size.max(size);
            chunks.push(Vec::with_capacity(cap));
        }
        let current_chunk = chunks.last_mut().unwrap();
        let current_len = current_chunk.len();
        current_chunk.reserve(size);
        unsafe {
            current_chunk.set_len(current_len + size);
            let ptr = current_chunk.as_mut_ptr().add(current_len);
            self.allocated_bytes.fetch_add(size, Ordering::Relaxed);
            ptr
        }
    }

    pub fn total_allocated_bytes(&self) -> usize {
        self.allocated_bytes.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        let mut chunks = self.chunks.lock();
        chunks.clear();
        self.allocated_bytes.store(0, Ordering::Relaxed);
    }
}

/// 06-098 & 06-099: Memory Accounting & Out-of-Memory Handling
pub struct MemoryTracker {
    limit_bytes: usize,
    used_bytes: AtomicUsize,
    peak_bytes: AtomicUsize,
}

impl MemoryTracker {
    pub fn new(limit_bytes: usize) -> Self {
        Self {
            limit_bytes,
            used_bytes: AtomicUsize::new(0),
            peak_bytes: AtomicUsize::new(0),
        }
    }

    pub fn allocate(&self, bytes: usize) -> Result<(), &'static str> {
        let current = self.used_bytes.load(Ordering::Relaxed);
        if current + bytes > self.limit_bytes {
            return Err("Err(OutOfMemory): allocation exceeds memory limit");
        }
        let new_val = self.used_bytes.fetch_add(bytes, Ordering::Relaxed) + bytes;
        let mut peak = self.peak_bytes.load(Ordering::Relaxed);
        while new_val > peak {
            match self.peak_bytes.compare_exchange_weak(peak, new_val, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }
        Ok(())
    }

    pub fn deallocate(&self, bytes: usize) {
        self.used_bytes.fetch_sub(bytes.min(self.used_bytes.load(Ordering::Relaxed)), Ordering::Relaxed);
    }

    pub fn used_bytes(&self) -> usize {
        self.used_bytes.load(Ordering::Relaxed)
    }

    pub fn peak_bytes(&self) -> usize {
        self.peak_bytes.load(Ordering::Relaxed)
    }

    pub fn limit_bytes(&self) -> usize {
        self.limit_bytes
    }
}

/// 06-100 & 06-101: File Descriptor Limits & Open-File Cache
pub struct OpenFileCache {
    max_open: usize,
    handles: Mutex<HashMap<u64, u32>>, // FileId -> dummy fd representation
    lru_order: Mutex<VecDeque<u64>>,
    opens_count: AtomicU64,
    closes_count: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl OpenFileCache {
    pub fn new(max_open: usize) -> Self {
        Self {
            max_open: max_open.max(1),
            handles: Mutex::new(HashMap::new()),
            lru_order: Mutex::new(VecDeque::new()),
            opens_count: AtomicU64::new(0),
            closes_count: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    pub fn get_or_open(&self, file_id: u64) -> Result<u32, &'static str> {
        let mut handles = self.handles.lock();
        let mut lru = self.lru_order.lock();

        if let Some(&fd) = handles.get(&file_id) {
            lru.retain(|&id| id != file_id);
            lru.push_back(file_id);
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(fd);
        }

        self.misses.fetch_add(1, Ordering::Relaxed);

        if handles.len() >= self.max_open {
            if let Some(oldest_id) = lru.pop_front() {
                handles.remove(&oldest_id);
                self.closes_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        let new_fd = (file_id + 100) as u32;
        handles.insert(file_id, new_fd);
        lru.push_back(file_id);
        self.opens_count.fetch_add(1, Ordering::Relaxed);
        Ok(new_fd)
    }

    pub fn current_open(&self) -> usize {
        self.handles.lock().len()
    }

    pub fn total_opens(&self) -> u64 {
        self.opens_count.load(Ordering::Relaxed)
    }

    pub fn total_closes(&self) -> u64 {
        self.closes_count.load(Ordering::Relaxed)
    }

    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }
}

/// 06-102: Read-Ahead Prefetch Policy for Sequential Scans
pub struct ReadAheadPolicy {
    sequential_threshold: usize,
    last_block_id: AtomicU64,
    sequential_count: AtomicUsize,
    prefetch_count: AtomicU64,
}

impl ReadAheadPolicy {
    pub fn new(sequential_threshold: usize) -> Self {
        Self {
            sequential_threshold,
            last_block_id: AtomicU64::new(u64::MAX),
            sequential_count: AtomicUsize::new(0),
            prefetch_count: AtomicU64::new(0),
        }
    }

    pub fn record_access(&self, block_id: u64) -> bool {
        let last = self.last_block_id.swap(block_id, Ordering::Relaxed);
        if last != u64::MAX && block_id == last + 1 {
            let seq = self.sequential_count.fetch_add(1, Ordering::Relaxed) + 1;
            if seq >= self.sequential_threshold {
                self.prefetch_count.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        } else {
            self.sequential_count.store(0, Ordering::Relaxed);
        }
        false
    }

    pub fn total_prefetches(&self) -> u64 {
        self.prefetch_count.load(Ordering::Relaxed)
    }
}

/// 06-103: Resource Quotas per Client
pub struct ResourceQuota {
    max_ops_per_window: u64,
    max_bytes_per_window: u64,
    window_duration: Duration,
    clients: Mutex<HashMap<String, (u64, u64, Instant)>>,
}

impl ResourceQuota {
    pub fn new(max_ops: u64, max_bytes: u64, window: Duration) -> Self {
        Self {
            max_ops_per_window: max_ops,
            max_bytes_per_window: max_bytes,
            window_duration: window,
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_and_consume(&self, client_id: &str, ops: u64, bytes: u64, now: Instant) -> Result<(), &'static str> {
        let mut clients = self.clients.lock();
        let entry = clients.entry(client_id.to_string()).or_insert((0, 0, now));
        if now.duration_since(entry.2) >= self.window_duration {
            entry.0 = 0;
            entry.1 = 0;
            entry.2 = now;
        }
        if entry.0 + ops > self.max_ops_per_window || entry.1 + bytes > self.max_bytes_per_window {
            return Err("Err(QuotaExceeded): client rate quota exceeded");
        }
        entry.0 += ops;
        entry.1 += bytes;
        Ok(())
    }
}

/// 06-104: Memory Pressure Telemetry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    Normal,
    Elevated,
    Critical,
}

pub struct MemoryPressureTelemetry {
    elevated_threshold_pct: f64,
    critical_threshold_pct: f64,
}

impl MemoryPressureTelemetry {
    pub fn new(elevated_pct: f64, critical_pct: f64) -> Self {
        Self {
            elevated_threshold_pct: elevated_pct,
            critical_threshold_pct: critical_pct,
        }
    }

    pub fn evaluate(&self, used_bytes: usize, total_bytes: usize) -> MemoryPressureLevel {
        if total_bytes == 0 {
            return MemoryPressureLevel::Normal;
        }
        let pct = (used_bytes as f64) / (total_bytes as f64) * 100.0;
        if pct >= self.critical_threshold_pct {
            MemoryPressureLevel::Critical
        } else if pct >= self.elevated_threshold_pct {
            MemoryPressureLevel::Elevated
        } else {
            MemoryPressureLevel::Normal
        }
    }
}

/// 06-133: Operation Metrics
#[derive(Default)]
pub struct OperationMetrics {
    pub puts: AtomicU64,
    pub gets: AtomicU64,
    pub deletes: AtomicU64,
    pub scans: AtomicU64,
    pub errors: AtomicU64,
}

impl OperationMetrics {
    pub fn record_op(&self, op: &str, is_err: bool) {
        match op {
            "put" => self.puts.fetch_add(1, Ordering::Relaxed),
            "get" => self.gets.fetch_add(1, Ordering::Relaxed),
            "delete" => self.deletes.fetch_add(1, Ordering::Relaxed),
            "scan" => self.scans.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };
        if is_err {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// 06-137: Latency Histograms
pub struct LatencyHistogram {
    buckets: [AtomicU64; 32], // Log2 buckets from 1µs to 2^31µs (~2147s)
}

impl LatencyHistogram {
    pub fn new() -> Self {
        Self {
            buckets: Default::default(),
        }
    }

    pub fn record(&self, latency_us: u64) {
        let bucket = if latency_us == 0 {
            0
        } else {
            (64 - latency_us.leading_zeros()).min(31) as usize
        };
        self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
    }

    pub fn percentile(&self, p: f64) -> u64 {
        let total: u64 = self.buckets.iter().map(|b| b.load(Ordering::Relaxed)).sum();
        if total == 0 {
            return 0;
        }
        let target = (total as f64 * p / 100.0).round() as u64;
        let mut accumulated = 0;
        for (i, b) in self.buckets.iter().enumerate() {
            accumulated += b.load(Ordering::Relaxed);
            if accumulated >= target {
                return if i == 0 { 1 } else { 1 << (i - 1) };
            }
        }
        1 << 30
    }
}

/// 06-143: Exportable Benchmark Results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkReport {
    pub schema_version: String,
    pub benchmark_name: String,
    pub total_ops: u64,
    pub ops_per_sec: f64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
}

impl BenchmarkReport {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn to_csv(&self) -> String {
        format!(
            "schema_version,benchmark_name,total_ops,ops_per_sec,p50_latency_us,p95_latency_us,p99_latency_us\n{},{},{},{:.2},{},{},{}",
            self.schema_version, self.benchmark_name, self.total_ops, self.ops_per_sec, self.p50_latency_us, self.p95_latency_us, self.p99_latency_us
        )
    }
}
