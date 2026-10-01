use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use crate::execution::vector::SelectionVector;

pub const DEFAULT_MORSEL_SIZE: usize = 10_000;

/// A Morsel represents a slice [start, end) of row offsets assigned to a worker thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Morsel {
    pub start: usize,
    pub end: usize,
}

impl Morsel {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// MorselScheduler dynamically distributes fixed-size morsels to worker threads using atomic fetch-add.
#[derive(Debug)]
pub struct MorselScheduler {
    total_rows: usize,
    morsel_size: usize,
    cursor: AtomicUsize,
}

impl MorselScheduler {
    pub fn new(total_rows: usize, morsel_size: usize) -> Self {
        Self {
            total_rows,
            morsel_size: morsel_size.max(1),
            cursor: AtomicUsize::new(0),
        }
    }

    /// Fetches the next available morsel. Returns None when all rows are dispatched.
    pub fn next_morsel(&self) -> Option<Morsel> {
        loop {
            let current = self.cursor.load(Ordering::Relaxed);
            if current >= self.total_rows {
                return None;
            }
            let next = (current + self.morsel_size).min(self.total_rows);
            if self.cursor.compare_exchange_weak(current, next, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                return Some(Morsel {
                    start: current,
                    end: next,
                });
            }
        }
    }

    pub fn reset(&self) {
        self.cursor.store(0, Ordering::SeqCst);
    }
}

/// Manages Rayon ThreadPool instances for parallel morsel execution.
pub struct MorselThreadPool {
    pub pool: rayon::ThreadPool,
    pub thread_count: usize,
}

impl MorselThreadPool {
    pub fn new(threads: usize) -> Self {
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|idx| format!("morsel-worker-{}", idx))
            .build()
            .unwrap_or_else(|_| ThreadPoolBuilder::new().build().unwrap());
        Self {
            pool,
            thread_count: threads,
        }
    }

    pub fn execute<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        self.pool.install(f)
    }
}

/// Executes parallel columnar scans over partitioned morsels.
pub struct ParallelScan;

impl ParallelScan {
    pub fn scan_filtered<F>(
        scheduler: &Arc<MorselScheduler>,
        pool: &MorselThreadPool,
        predicate: F,
    ) -> SelectionVector
    where
        F: Fn(usize) -> bool + Sync + Send,
    {
        pool.execute(|| {
            let morsels: Vec<Morsel> = {
                let mut list = Vec::new();
                while let Some(m) = scheduler.next_morsel() {
                    list.push(m);
                }
                list
            };

            let partial_selections: Vec<Vec<usize>> = morsels
                .par_iter()
                .map(|morsel| {
                    let mut matches = Vec::with_capacity(morsel.len() / 2);
                    for row in morsel.start..morsel.end {
                        if predicate(row) {
                            matches.push(row);
                        }
                    }
                    matches
                })
                .collect();

            let total_matches: usize = partial_selections.iter().map(|v| v.len()).sum();
            let mut combined = Vec::with_capacity(total_matches);
            for mut partial in partial_selections {
                combined.append(&mut partial);
            }
            SelectionVector {
                indices: combined,
            }
        })
    }
}

/// Executes parallel grouped aggregations with thread-local hash maps and final combine.
pub struct ParallelAggregation;

impl ParallelAggregation {
    pub fn sum_grouped_i64<K, FKey, FVal>(
        scheduler: &Arc<MorselScheduler>,
        pool: &MorselThreadPool,
        get_key: FKey,
        get_val: FVal,
    ) -> HashMap<K, i64>
    where
        K: std::hash::Hash + Eq + Send + Sync + Clone,
        FKey: Fn(usize) -> K + Sync + Send,
        FVal: Fn(usize) -> i64 + Sync + Send,
    {
        pool.execute(|| {
            let morsels: Vec<Morsel> = {
                let mut list = Vec::new();
                while let Some(m) = scheduler.next_morsel() {
                    list.push(m);
                }
                list
            };

            let partial_maps: Vec<HashMap<K, i64>> = morsels
                .par_iter()
                .map(|morsel| {
                    let mut local_map: HashMap<K, i64> = HashMap::new();
                    for row in morsel.start..morsel.end {
                        let k = get_key(row);
                        let v = get_val(row);
                        *local_map.entry(k).or_insert(0) += v;
                    }
                    local_map
                })
                .collect();

            let mut final_map: HashMap<K, i64> = HashMap::new();
            for pmap in partial_maps {
                for (k, v) in pmap {
                    *final_map.entry(k).or_insert(0) += v;
                }
            }
            final_map
        })
    }
}

/// Executes parallel hash join with shared hash table build and partitioned morsel probe.
pub struct ParallelHashJoin;

impl ParallelHashJoin {
    pub fn inner_join_int<FBuildKey, FBuildVal, FProbeKey, FProbeVal>(
        build_rows: usize,
        probe_rows: usize,
        pool: &MorselThreadPool,
        get_build_key: FBuildKey,
        get_build_val: FBuildVal,
        get_probe_key: FProbeKey,
        get_probe_val: FProbeVal,
    ) -> Vec<(i64, i64)>
    where
        FBuildKey: Fn(usize) -> i64 + Sync + Send,
        FBuildVal: Fn(usize) -> i64 + Sync + Send,
        FProbeKey: Fn(usize) -> i64 + Sync + Send,
        FProbeVal: Fn(usize) -> i64 + Sync + Send,
    {
        pool.execute(|| {
            // Build phase
            let mut build_table: HashMap<i64, Vec<i64>> = HashMap::new();
            for r in 0..build_rows {
                let k = get_build_key(r);
                let v = get_build_val(r);
                build_table.entry(k).or_default().push(v);
            }

            let build_table = Arc::new(build_table);
            let scheduler = Arc::new(MorselScheduler::new(probe_rows, DEFAULT_MORSEL_SIZE));

            let morsels: Vec<Morsel> = {
                let mut list = Vec::new();
                while let Some(m) = scheduler.next_morsel() {
                    list.push(m);
                }
                list
            };

            // Probe phase across morsels
            let partial_matches: Vec<Vec<(i64, i64)>> = morsels
                .par_iter()
                .map(|morsel| {
                    let mut local_matches = Vec::new();
                    for r in morsel.start..morsel.end {
                        let pk = get_probe_key(r);
                        let pv = get_probe_val(r);
                        if let Some(bvals) = build_table.get(&pk) {
                            for &bv in bvals {
                                local_matches.push((bv, pv));
                            }
                        }
                    }
                    local_matches
                })
                .collect();

            let total_len: usize = partial_matches.iter().map(|v| v.len()).sum();
            let mut result = Vec::with_capacity(total_len);
            for mut p in partial_matches {
                result.append(&mut p);
            }
            result
        })
    }
}
