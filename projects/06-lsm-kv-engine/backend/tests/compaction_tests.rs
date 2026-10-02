use std::fs;
use std::path::Path;
use tempfile::tempdir;
use lsm_kv_engine::compaction::{FileMetadata, LeveledCompactionController};
use lsm_kv_engine::sstable::{SsTableBuilder, SsTableReader};

// FEAT: 06-053 "Leveled compaction"
// ACCEPT: Leveled compaction preserves logical state before == after across multi-level SSTable merges
#[test]
fn test_leveled_compaction_state_preservation() {
    let dir = tempdir().unwrap();
    let controller = LeveledCompactionController::new(2);

    // Create 2 L0 SSTables
    let sst1_path = dir.path().join("000001.sst");
    let mut b1 = SsTableBuilder::new(&sst1_path, 10);
    b1.add(b"key_a", Some(b"val_a1"));
    b1.add(b"key_c", Some(b"val_c1"));
    let _ = b1.finish().unwrap();

    let sst2_path = dir.path().join("000002.sst");
    let mut b2 = SsTableBuilder::new(&sst2_path, 10);
    b2.add(b"key_b", Some(b"val_b1"));
    b2.add(b"key_c", Some(b"val_c2_newer")); // Overrides key_c
    let _ = b2.finish().unwrap();

    let f1 = FileMetadata {
        file_number: 1,
        level: 0,
        path: sst1_path,
        size_bytes: 100,
        first_key: b"key_a".to_vec(),
        last_key: b"key_c".to_vec(),
    };
    let f2 = FileMetadata {
        file_number: 2,
        level: 0,
        path: sst2_path,
        size_bytes: 100,
        first_key: b"key_b".to_vec(),
        last_key: b"key_c".to_vec(),
    };

    let task = controller.pick_compaction(&[f1, f2], &[]).unwrap();
    let (new_files, obsolete) = controller.execute_compaction(&task, dir.path(), 3).unwrap();

    assert_eq!(new_files.len(), 1);
    assert_eq!(obsolete.len(), 2);

    let reader = SsTableReader::open(&new_files[0].path).unwrap();
    assert_eq!(reader.get(b"key_a").unwrap(), Some(Some(b"val_a1".to_vec())));
    assert_eq!(reader.get(b"key_b").unwrap(), Some(Some(b"val_b1".to_vec())));
    assert_eq!(reader.get(b"key_c").unwrap(), Some(Some(b"val_c2_newer".to_vec()))); // Newer L0 entry overrides older
}

// FEAT: 06-055 "Compaction picker"
// ACCEPT: Compaction picker detects L0 threshold triggers and identifies overlapping L1 target files
#[test]
fn test_compaction_picker_overlapping_key_ranges() {
    let dir = tempdir().unwrap();
    let controller = LeveledCompactionController::new(2);

    let l0_file1 = FileMetadata {
        file_number: 1,
        level: 0,
        path: dir.path().join("000001.sst"),
        size_bytes: 100,
        first_key: b"m".to_vec(),
        last_key: b"z".to_vec(),
    };
    let l0_file2 = FileMetadata {
        file_number: 2,
        level: 0,
        path: dir.path().join("000002.sst"),
        size_bytes: 100,
        first_key: b"a".to_vec(),
        last_key: b"k".to_vec(),
    };

    let l1_overlap = FileMetadata {
        file_number: 10,
        level: 1,
        path: dir.path().join("000010.sst"),
        size_bytes: 200,
        first_key: b"b".to_vec(),
        last_key: b"f".to_vec(),
    };
    let l1_non_overlap = FileMetadata {
        file_number: 11,
        level: 1,
        path: dir.path().join("000011.sst"),
        size_bytes: 200,
        first_key: b"zzz_outside".to_vec(),
        last_key: b"zzz_max".to_vec(),
    };

    let task = controller.pick_compaction(&[l0_file1, l0_file2], &[l1_overlap.clone(), l1_non_overlap]).unwrap();
    assert_eq!(task.from_level, 0);
    assert_eq!(task.to_level, 1);
    assert_eq!(task.input_level_files.len(), 2);
    assert_eq!(task.target_level_files.len(), 1);
    assert_eq!(task.target_level_files[0].file_number, 10);
}

// FEAT: 06-056 "Compaction scheduling"
// ACCEPT: Compaction scheduling triggers only when level file count exceeds threshold
#[test]
fn test_compaction_scheduling_threshold_guards() {
    use std::collections::HashSet;

    let controller = LeveledCompactionController::new(4);
    let l0_file = FileMetadata {
        file_number: 1,
        level: 0,
        path: Path::new("1.sst").to_path_buf(),
        size_bytes: 1000,
        first_key: b"a".to_vec(),
        last_key: b"b".to_vec(),
    };

    // 1. L0 file count threshold trigger
    let l0_under = vec![l0_file.clone(), l0_file.clone(), l0_file.clone()]; // 3 files < 4
    assert_eq!(controller.compute_compaction_score(0, &l0_under), 0.75);
    assert!(controller.pick_compaction(&l0_under, &[]).is_none());

    let l0_over = vec![l0_file.clone(), l0_file.clone(), l0_file.clone(), l0_file.clone(), l0_file.clone()]; // 5 files >= 4
    assert_eq!(controller.compute_compaction_score(0, &l0_over), 1.25);
    assert!(controller.pick_compaction(&l0_over, &[]).is_some());

    // 2. Level-size-ratio score trigger for L1 (target: 10MB) and L2 (target: 100MB)
    let l1_files = vec![FileMetadata {
        file_number: 10,
        level: 1,
        path: Path::new("10.sst").to_path_buf(),
        size_bytes: 15 * 1024 * 1024, // 15MB (score = 1.5)
        first_key: b"a".to_vec(),
        last_key: b"z".to_vec(),
    }];
    assert_eq!(controller.compute_compaction_score(1, &l1_files), 1.5);

    let l2_files = vec![FileMetadata {
        file_number: 20,
        level: 2,
        path: Path::new("20.sst").to_path_buf(),
        size_bytes: 200 * 1024 * 1024, // 200MB (L2 target is 100MB, score = 2.0)
        first_key: b"a".to_vec(),
        last_key: b"z".to_vec(),
    }];
    assert_eq!(controller.compute_compaction_score(2, &l2_files), 2.0);

    // Exact score = 0.9 under-threshold check (should not trigger compaction)
    let l1_under = vec![FileMetadata {
        file_number: 12,
        level: 1,
        path: Path::new("12.sst").to_path_buf(),
        size_bytes: 9 * 1024 * 1024, // 9MB -> score 0.9
        first_key: b"a".to_vec(),
        last_key: b"z".to_vec(),
    }];
    assert_eq!(controller.compute_compaction_score(1, &l1_under), 0.9);
    let levels_under = vec![vec![], l1_under];
    let busy_empty = HashSet::new();
    assert_eq!(controller.pick_best_level_to_compact(&levels_under, &busy_empty), None);

    // Score = 1.0 threshold trigger
    let l1_exact = vec![FileMetadata {
        file_number: 13,
        level: 1,
        path: Path::new("13.sst").to_path_buf(),
        size_bytes: 10 * 1024 * 1024, // exactly 10MB -> score 1.0
        first_key: b"a".to_vec(),
        last_key: b"z".to_vec(),
    }];
    assert_eq!(controller.compute_compaction_score(1, &l1_exact), 1.0);
    let levels_exact = vec![vec![], l1_exact];
    assert_eq!(controller.pick_best_level_to_compact(&levels_exact, &busy_empty), Some(1));

    // 3. Highest-score-first pick (L2 score 2.0 > L1 score 1.5 > L0 score 1.25 -> picks L2)
    let levels = vec![l0_over.clone(), l1_files.clone(), l2_files.clone()];
    assert_eq!(controller.pick_best_level_to_compact(&levels, &busy_empty), Some(2));

    // 4. No two concurrent compactions on the same level (guard against busy level 2)
    let mut busy = HashSet::new();
    busy.insert(2);
    assert_eq!(controller.pick_best_level_to_compact(&levels, &busy), Some(1)); // Falls back to L1
    busy.insert(1);
    assert_eq!(controller.pick_best_level_to_compact(&levels, &busy), Some(0)); // Falls back to L0

    // Test pick_compaction boundary conditions with >= threshold L0 files
    let l0_f1 = FileMetadata {
        file_number: 101,
        level: 0,
        path: Path::new("101.sst").to_path_buf(),
        size_bytes: 1000,
        first_key: b"m".to_vec(),
        last_key: b"p".to_vec(),
    };
    let l0_f2 = FileMetadata {
        file_number: 102,
        level: 0,
        path: Path::new("102.sst").to_path_buf(),
        size_bytes: 1000,
        first_key: b"c".to_vec(), // smaller than first_key of f1
        last_key: b"e".to_vec(),
    };
    let l0_f3 = FileMetadata {
        file_number: 103,
        level: 0,
        path: Path::new("103.sst").to_path_buf(),
        size_bytes: 1000,
        first_key: b"f".to_vec(),
        last_key: b"t".to_vec(), // larger than last_key of f1
    };
    let l0_f4 = FileMetadata {
        file_number: 104,
        level: 0,
        path: Path::new("104.sst").to_path_buf(),
        size_bytes: 1000,
        first_key: b"g".to_vec(),
        last_key: b"h".to_vec(),
    };

    // Target files in L1:
    // Before min_key (c): "a".."b" -> no overlap
    let l1_before = FileMetadata {
        file_number: 201,
        level: 1,
        path: Path::new("201.sst").to_path_buf(),
        size_bytes: 5000,
        first_key: b"a".to_vec(),
        last_key: b"b".to_vec(),
    };
    // Overlapping min_key (c): "b".."d" -> overlap
    let l1_overlap_left = FileMetadata {
        file_number: 202,
        level: 1,
        path: Path::new("202.sst").to_path_buf(),
        size_bytes: 5000,
        first_key: b"b".to_vec(),
        last_key: b"d".to_vec(),
    };
    // Overlapping max_key (t): "s".."v" -> overlap
    let l1_overlap_right = FileMetadata {
        file_number: 203,
        level: 1,
        path: Path::new("203.sst").to_path_buf(),
        size_bytes: 5000,
        first_key: b"s".to_vec(),
        last_key: b"v".to_vec(),
    };
    // After max_key (t): "u".."z" -> no overlap
    let l1_after = FileMetadata {
        file_number: 204,
        level: 1,
        path: Path::new("204.sst").to_path_buf(),
        size_bytes: 5000,
        first_key: b"u".to_vec(),
        last_key: b"z".to_vec(),
    };

    let l0_inputs = vec![l0_f1, l0_f2, l0_f3, l0_f4];
    let l1_targets = vec![l1_before, l1_overlap_left, l1_overlap_right, l1_after];
    let task = controller.pick_compaction(&l0_inputs, &l1_targets).unwrap();
    assert_eq!(task.from_level, 0);
    assert_eq!(task.to_level, 1);
    assert_eq!(task.input_level_files.len(), 4);
    assert_eq!(task.target_level_files.len(), 2);
    assert_eq!(task.target_level_files[0].file_number, 202);
    assert_eq!(task.target_level_files[1].file_number, 203);
}

// FEAT: 06-059 "Compaction output generation"
// ACCEPT: Compaction output generation writes new sorted SSTable and cleans up obsolete input files
#[test]
fn test_compaction_output_generation_and_cleanup() {
    let dir = tempdir().unwrap();
    let controller = LeveledCompactionController::new(1);

    let sst_path = dir.path().join("000001.sst");
    let mut b = SsTableBuilder::new(&sst_path, 10);
    b.add(b"alpha", Some(b"100"));
    let _ = b.finish().unwrap();

    let f1 = FileMetadata {
        file_number: 1,
        level: 0,
        path: sst_path.clone(),
        size_bytes: 100,
        first_key: b"alpha".to_vec(),
        last_key: b"alpha".to_vec(),
    };

    let initial_read = controller.stats.bytes_read.load(std::sync::atomic::Ordering::Relaxed);
    let initial_written = controller.stats.bytes_written.load(std::sync::atomic::Ordering::Relaxed);

    let task = controller.pick_compaction(&[f1], &[]).unwrap();
    let (outputs, obsolete) = controller.execute_compaction(&task, dir.path(), 2).unwrap();

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].file_number, 2);
    assert!(!sst_path.exists()); // Obsolete file removed
    assert!(outputs[0].path.exists()); // Output file created

    let final_read = controller.stats.bytes_read.load(std::sync::atomic::Ordering::Relaxed);
    let final_written = controller.stats.bytes_written.load(std::sync::atomic::Ordering::Relaxed);
    assert!(final_read > initial_read);
    assert!(final_written > initial_written);
}

// FEAT: 06-060 "Write amplification accounting"
// ACCEPT: Write amplification tracks ratio of total bytes written to bytes read across compactions
#[test]
fn test_write_amplification_accounting_metrics() {
    let dir = tempdir().unwrap();
    let controller = LeveledCompactionController::new(1);

    // Initial state: 0 read bytes -> default write amp is 1.0
    assert_eq!(controller.stats.write_amplification(), 1.0);

    // Direct arithmetic check with non-trivial values (e.g. 350 / 100 = 3.5 != 350 * 100)
    controller.stats.bytes_read.store(100, std::sync::atomic::Ordering::Relaxed);
    controller.stats.bytes_written.store(350, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(controller.stats.write_amplification(), 3.5);

    let sst_path = dir.path().join("000001.sst");
    let mut b = SsTableBuilder::new(&sst_path, 10);
    b.add(b"key1", Some(b"val1"));
    let _ = b.finish().unwrap();

    let meta = fs::metadata(&sst_path).unwrap();
    let f1 = FileMetadata {
        file_number: 1,
        level: 0,
        path: sst_path,
        size_bytes: meta.len(),
        first_key: b"key1".to_vec(),
        last_key: b"key1".to_vec(),
    };

    let task = controller.pick_compaction(&[f1], &[]).unwrap();
    let _ = controller.execute_compaction(&task, dir.path(), 2).unwrap();

    let write_amp = controller.stats.write_amplification();
    assert!(write_amp > 0.0);
    assert_eq!(controller.stats.compactions_completed.load(std::sync::atomic::Ordering::Relaxed), 1);
}
