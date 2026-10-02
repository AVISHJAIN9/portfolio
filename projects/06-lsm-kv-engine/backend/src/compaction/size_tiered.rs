use crate::compaction::leveled::{CompactionTask, FileMetadata};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Strategy trait for compaction algorithms (Leveled, Size-Tiered, FIFO, etc.)
pub trait CompactionStrategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn pick_compaction(&self, levels: &[Vec<FileMetadata>]) -> Option<CompactionTask>;
}

/// Size-Tiered Compaction Controller
/// Groups files into size buckets and triggers compaction when a bucket accumulates >= min_threshold files.
pub struct SizeTieredCompactionController {
    pub min_threshold: usize,
    pub max_threshold: usize,
    pub size_ratio: f64,
}

impl SizeTieredCompactionController {
    pub fn new(min_threshold: usize, max_threshold: usize, size_ratio: f64) -> Self {
        Self {
            min_threshold,
            max_threshold,
            size_ratio,
        }
    }

    /// Groups files into size tiers where each file size is within `size_ratio` of the tier's average.
    pub fn group_into_tiers<'a>(&self, files: &'a [FileMetadata]) -> Vec<Vec<&'a FileMetadata>> {
        let mut sorted = files.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|f| f.size_bytes);

        let mut tiers: Vec<Vec<&'a FileMetadata>> = Vec::new();
        for file in sorted {
            let mut added = false;
            for tier in &mut tiers {
                let avg_size: u64 = tier.iter().map(|f| f.size_bytes).sum::<u64>() / tier.len() as u64;
                let min_bound = (avg_size as f64 / self.size_ratio) as u64;
                let max_bound = (avg_size as f64 * self.size_ratio) as u64;
                if file.size_bytes >= min_bound && file.size_bytes <= max_bound {
                    tier.push(file);
                    added = true;
                    break;
                }
            }
            if !added {
                tiers.push(vec![file]);
            }
        }
        tiers
    }

    /// Pick compaction candidate: finds the first tier with >= min_threshold files and picks up to max_threshold files.
    pub fn pick_candidate_files(&self, files: &[FileMetadata]) -> Option<Vec<FileMetadata>> {
        let tiers = self.group_into_tiers(files);
        for tier in tiers {
            if tier.len() >= self.min_threshold {
                let to_pick = tier.into_iter().take(self.max_threshold).cloned().collect::<Vec<_>>();
                return Some(to_pick);
            }
        }
        None
    }
}

impl CompactionStrategy for SizeTieredCompactionController {
    fn name(&self) -> &'static str {
        "size-tiered"
    }

    fn pick_compaction(&self, levels: &[Vec<FileMetadata>]) -> Option<CompactionTask> {
        let all_files: Vec<FileMetadata> = levels.iter().flatten().cloned().collect();
        let candidate = self.pick_candidate_files(&all_files)?;
        Some(CompactionTask {
            from_level: 0,
            to_level: 1,
            input_level_files: candidate,
            target_level_files: Vec::new(),
        })
    }
}

/// Compaction Debt Tracker (06-062)
/// Computes pending compaction debt across all levels to guide backpressure and scheduling.
pub struct CompactionDebtTracker {
    pub base_level_bytes: u64,
    pub level_multiplier: u64,
    pub l0_file_num_compaction_trigger: usize,
    pub avg_l0_file_size: u64,
}

impl CompactionDebtTracker {
    pub fn new(base_level_bytes: u64, level_multiplier: u64, l0_trigger: usize, avg_l0_file_size: u64) -> Self {
        Self {
            base_level_bytes,
            level_multiplier,
            l0_file_num_compaction_trigger: l0_trigger,
            avg_l0_file_size,
        }
    }

    /// Calculates total pending debt in bytes across all levels.
    pub fn compute_total_debt(&self, levels: &[Vec<FileMetadata>]) -> u64 {
        let mut total_debt = 0u64;

        if let Some(l0) = levels.first() {
            if l0.len() > self.l0_file_num_compaction_trigger {
                let excess_files = (l0.len() - self.l0_file_num_compaction_trigger) as u64;
                total_debt += excess_files * self.avg_l0_file_size;
            }
        }

        for (lvl_idx, files) in levels.iter().enumerate().skip(1) {
            let target_capacity = self.base_level_bytes * self.level_multiplier.pow((lvl_idx - 1) as u32);
            let current_bytes: u64 = files.iter().map(|f| f.size_bytes).sum();
            if current_bytes > target_capacity {
                total_debt += current_bytes - target_capacity;
            }
        }

        total_debt
    }
}

/// Injectable Clock trait for deterministic testing (06-063)
pub trait Clock: Send + Sync {
    fn now_micros(&self) -> u64;
}

#[derive(Default)]
pub struct MockClock {
    pub current_micros: AtomicU64,
}

impl MockClock {
    pub fn new(start_micros: u64) -> Self {
        Self {
            current_micros: AtomicU64::new(start_micros),
        }
    }
    pub fn advance_micros(&self, delta: u64) {
        self.current_micros.fetch_add(delta, Ordering::SeqCst);
    }
}

impl Clock for MockClock {
    fn now_micros(&self) -> u64 {
        self.current_micros.load(Ordering::SeqCst)
    }
}

/// Compaction Throttler / Rate Limiter (06-063)
/// Enforces max bytes/sec write limit with injected clock.
pub struct CompactionThrottler {
    rate_bytes_per_sec: u64,
    clock: Arc<dyn Clock>,
    last_refill_micros: AtomicU64,
    tokens: AtomicU64,
}

impl CompactionThrottler {
    pub fn new(rate_bytes_per_sec: u64, clock: Arc<dyn Clock>) -> Self {
        let now = clock.now_micros();
        Self {
            rate_bytes_per_sec,
            clock,
            last_refill_micros: AtomicU64::new(now),
            tokens: AtomicU64::new(rate_bytes_per_sec), // Start full
        }
    }

    /// Try to consume bytes; returns Ok(delay_needed_micros) or Ok(0) if immediately allowed.
    pub fn request_tokens(&self, requested_bytes: u64) -> u64 {
        let now = self.clock.now_micros();
        let last = self.last_refill_micros.load(Ordering::SeqCst);
        let elapsed_micros = now.saturating_sub(last);

        // Refill tokens
        let new_tokens = (elapsed_micros as u128 * self.rate_bytes_per_sec as u128 / 1_000_000) as u64;
        let current = self.tokens.load(Ordering::SeqCst);
        let updated = (current + new_tokens).min(self.rate_bytes_per_sec);
        self.tokens.store(updated, Ordering::SeqCst);
        self.last_refill_micros.store(now, Ordering::SeqCst);

        let cur_tokens = self.tokens.load(Ordering::SeqCst);
        if cur_tokens >= requested_bytes {
            self.tokens.store(cur_tokens - requested_bytes, Ordering::SeqCst);
            0
        } else {
            let deficit = requested_bytes - cur_tokens;
            self.tokens.store(0, Ordering::SeqCst);
            // Time needed to accumulate deficit
            ((deficit as u128 * 1_000_000) / self.rate_bytes_per_sec as u128) as u64
        }
    }
}

/// Concurrency & Conflict Tracker for Compactions (06-057)
#[derive(Default)]
pub struct CompactionConcurrencyManager {
    busy_levels: parking_lot::Mutex<std::collections::HashSet<usize>>,
    active_ranges: parking_lot::Mutex<Vec<(Vec<u8>, Vec<u8>)>>,
}

impl CompactionConcurrencyManager {
    pub fn try_lock_compaction(&self, level: usize, start_key: &[u8], end_key: &[u8]) -> bool {
        let mut busy = self.busy_levels.lock();
        let mut ranges = self.active_ranges.lock();

        if busy.contains(&level) {
            return false;
        }

        // Check overlapping key ranges across levels
        for (active_start, active_end) in ranges.iter() {
            if !(end_key < active_start.as_slice() || start_key > active_end.as_slice()) {
                return false;
            }
        }

        busy.insert(level);
        ranges.push((start_key.to_vec(), end_key.to_vec()));
        true
    }

    pub fn unlock_compaction(&self, level: usize, start_key: &[u8], end_key: &[u8]) {
        let mut busy = self.busy_levels.lock();
        let mut ranges = self.active_ranges.lock();
        busy.remove(&level);
        ranges.retain(|(s, e)| s.as_slice() != start_key || e.as_slice() != end_key);
    }
}

/// Min-Overlap Heuristic for Compaction Input Selection (06-058)
pub fn select_l0_file_with_min_overlap(
    l0_files: &[FileMetadata],
    l1_files: &[FileMetadata],
) -> Option<FileMetadata> {
    if l0_files.is_empty() {
        return None;
    }

    let mut best_file = None;
    let mut min_overlap_bytes = u64::MAX;

    for candidate in l0_files {
        let overlap_bytes: u64 = l1_files
            .iter()
            .filter(|l1| !(l1.last_key < candidate.first_key || l1.first_key > candidate.last_key))
            .map(|l1| l1.size_bytes)
            .sum();

        if overlap_bytes < min_overlap_bytes {
            min_overlap_bytes = overlap_bytes;
            best_file = Some(candidate.clone());
        }
    }

    best_file
}

/// JSON-serializable Compaction Benchmark Output (06-065)
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CompactionBenchReport {
    pub benchmark_name: String,
    pub duration_ms: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub write_amplification: f64,
    pub throughput_mb_s: f64,
}
