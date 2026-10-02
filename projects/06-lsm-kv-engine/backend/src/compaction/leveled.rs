use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use crate::sstable::{SsTableBuilder, SsTableReader};

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_number: u64,
    pub level: usize,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub first_key: Vec<u8>,
    pub last_key: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CompactionTask {
    pub from_level: usize,
    pub to_level: usize,
    pub input_level_files: Vec<FileMetadata>,
    pub target_level_files: Vec<FileMetadata>,
}

#[derive(Debug, Default)]
pub struct CompactionStats {
    pub bytes_read: AtomicU64,
    pub bytes_written: AtomicU64,
    pub compactions_completed: AtomicU64,
}

impl CompactionStats {
    pub fn write_amplification(&self) -> f64 {
        let read = self.bytes_read.load(Ordering::Relaxed) as f64;
        let written = self.bytes_written.load(Ordering::Relaxed) as f64;
        if read == 0.0 {
            1.0
        } else {
            written / read
        }
    }
}

pub struct LeveledCompactionController {
    pub l0_compaction_threshold: usize,
    pub level_size_multiplier: usize,
    pub max_bytes_for_level_base: u64,
    pub stats: Arc<CompactionStats>,
}

impl LeveledCompactionController {
    pub fn new(l0_threshold: usize) -> Self {
        Self {
            l0_compaction_threshold: l0_threshold,
            level_size_multiplier: 10,
            max_bytes_for_level_base: 10 * 1024 * 1024, // 10 MB
            stats: Arc::new(CompactionStats::default()),
        }
    }

    /// Computes compaction priority score for a given level
    pub fn compute_compaction_score(&self, level: usize, files: &[FileMetadata]) -> f64 {
        if level == 0 {
            files.len() as f64 / self.l0_compaction_threshold.max(1) as f64
        } else {
            let total_bytes: u64 = files.iter().map(|f| f.size_bytes).sum();
            let target_bytes = self.max_bytes_for_level_base * (self.level_size_multiplier.pow(level as u32 - 1) as u64);
            total_bytes as f64 / target_bytes as f64
        }
    }

    /// Selects the highest scoring eligible level for compaction, guarding against concurrent busy levels
    pub fn pick_best_level_to_compact(
        &self,
        levels: &[Vec<FileMetadata>],
        busy_levels: &HashSet<usize>,
    ) -> Option<usize> {
        let mut best_level = None;
        let mut max_score = 0.999; // Only schedule if score >= 1.0

        for (lvl, files) in levels.iter().enumerate() {
            if busy_levels.contains(&lvl) {
                continue;
            }
            let score = self.compute_compaction_score(lvl, files);
            if score > max_score {
                max_score = score;
                best_level = Some(lvl);
            }
        }

        best_level
    }

    /// Compaction picker: analyzes level files and picks compaction candidate inputs.
    pub fn pick_compaction(
        &self,
        l0_files: &[FileMetadata],
        l1_files: &[FileMetadata],
    ) -> Option<CompactionTask> {
        if l0_files.len() < self.l0_compaction_threshold {
            return None;
        }

        // Pick all L0 files
        let mut min_key = l0_files[0].first_key.clone();
        let mut max_key = l0_files[0].last_key.clone();

        for f in l0_files {
            if f.first_key < min_key {
                min_key = f.first_key.clone();
            }
            if f.last_key > max_key {
                max_key = f.last_key.clone();
            }
        }

        // Find overlapping L1 files
        let mut overlapping_l1 = Vec::new();
        for f in l1_files {
            if !(f.last_key < min_key || f.first_key > max_key) {
                overlapping_l1.push(f.clone());
            }
        }

        Some(CompactionTask {
            from_level: 0,
            to_level: 1,
            input_level_files: l0_files.to_vec(),
            target_level_files: overlapping_l1,
        })
    }

    /// Executes compaction task, merges sorted entries, creates new SSTables, and deletes obsolete inputs.
    pub fn execute_compaction(
        &self,
        task: &CompactionTask,
        out_dir: &Path,
        next_file_num: u64,
    ) -> std::io::Result<(Vec<FileMetadata>, Vec<PathBuf>)> {
        let mut merged_entries: BTreeMap<Vec<u8>, Option<Vec<u8>>> = BTreeMap::new();
        let mut total_bytes_read = 0u64;
        let mut obsolete_paths = Vec::new();

        // Read all from_level inputs
        for f in &task.input_level_files {
            total_bytes_read += f.size_bytes;
            obsolete_paths.push(f.path.clone());
            let reader = SsTableReader::open(&f.path)?;
            let entries = reader.scan_all()?;
            for (k, v) in entries {
                merged_entries.insert(k, v);
            }
        }

        // Read all target_level inputs
        for f in &task.target_level_files {
            total_bytes_read += f.size_bytes;
            obsolete_paths.push(f.path.clone());
            let reader = SsTableReader::open(&f.path)?;
            let entries = reader.scan_all()?;
            for (k, v) in entries {
                // If not already overridden by newer level 0 entry
                merged_entries.entry(k).or_insert(v);
            }
        }

        self.stats.bytes_read.fetch_add(total_bytes_read, Ordering::Relaxed);

        // Write new SSTable at target level
        let out_path = out_dir.join(format!("{:06}.sst", next_file_num));
        let mut builder = SsTableBuilder::new(&out_path, 100);

        for (k, v) in &merged_entries {
            builder.add(k, v.as_deref());
        }

        let reader = builder.finish()?;
        let meta = fs::metadata(&out_path)?;
        let bytes_written = meta.len();
        self.stats.bytes_written.fetch_add(bytes_written, Ordering::Relaxed);
        self.stats.compactions_completed.fetch_add(1, Ordering::Relaxed);

        // Delete obsolete files
        for p in &obsolete_paths {
            let _ = fs::remove_file(p);
        }

        let first_key = merged_entries.keys().next().cloned().unwrap_or_default();
        let last_key = merged_entries.keys().next_back().cloned().unwrap_or_default();

        let new_file_meta = FileMetadata {
            file_number: next_file_num,
            level: task.to_level,
            path: out_path,
            size_bytes: bytes_written,
            first_key,
            last_key,
        };

        Ok((vec![new_file_meta], obsolete_paths))
    }
}
