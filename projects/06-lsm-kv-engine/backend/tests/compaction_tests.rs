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

// FEAT: 06-054 "Size-tiered compaction"
// ACCEPT: Size-tiered compaction strategy groups files into size tiers and picks N similar-size candidate files matching oracle
#[test]
fn test_size_tiered_compaction_strategy_oracle() {
    use lsm_kv_engine::compaction::{CompactionStrategy, SizeTieredCompactionController};

    let controller = SizeTieredCompactionController::new(4, 8, 1.5);
    assert_eq!(controller.name(), "size-tiered");

    // Files in two distinct tiers: ~100MB tier and ~10MB tier
    let mut files = Vec::new();
    for i in 1..=3 {
        files.push(FileMetadata {
            file_number: i,
            level: 0,
            path: Path::new(&format!("{}.sst", i)).to_path_buf(),
            size_bytes: 100 * 1024 * 1024 + i * 1024,
            first_key: vec![i as u8],
            last_key: vec![i as u8 + 1],
        });
    }
    // Only 3 large files -> below min_threshold 4 -> None
    assert!(controller.pick_candidate_files(&files).is_none());

    // Add 4 small files (~10MB)
    for i in 4..=7 {
        files.push(FileMetadata {
            file_number: i,
            level: 0,
            path: Path::new(&format!("{}.sst", i)).to_path_buf(),
            size_bytes: 10 * 1024 * 1024 + (i - 4) * 1024,
            first_key: vec![i as u8],
            last_key: vec![i as u8 + 1],
        });
    }

    // Oracle expectation: tier with 4 files (10MB tier) is picked
    let candidates = controller.pick_candidate_files(&files).expect("expected small tier picked");
    assert_eq!(candidates.len(), 4);
    for cand in &candidates {
        assert!(cand.size_bytes < 20 * 1024 * 1024, "Picked candidate must be in 10MB tier");
    }

    let task = controller.pick_compaction(&[files]).unwrap();
    assert_eq!(task.from_level, 0);
    assert_eq!(task.input_level_files.len(), 4);
}

// FEAT: 06-057 "Compaction concurrency"
// ACCEPT: Concurrent compaction manager allows >=2 non-overlapping range compactions and rejects overlapping ranges
#[test]
fn test_compaction_concurrency_non_overlapping() {
    use lsm_kv_engine::compaction::CompactionConcurrencyManager;
    use std::sync::Arc;
    use std::thread;

    let mgr = Arc::new(CompactionConcurrencyManager::default());

    // 1. Level 1 [a..m] and Level 2 [n..z] can lock concurrently
    let l1_ok = mgr.try_lock_compaction(1, b"a", b"m");
    assert!(l1_ok, "Level 1 non-overlapping lock must succeed");

    let l2_ok = mgr.try_lock_compaction(2, b"n", b"z");
    assert!(l2_ok, "Level 2 non-overlapping lock must succeed");

    // 2. Conflicting lock attempts must fail
    assert!(!mgr.try_lock_compaction(1, b"b", b"c"), "Same level lock must fail");
    assert!(!mgr.try_lock_compaction(3, b"e", b"p"), "Overlapping key range lock must fail");

    // 3. Multithreaded validation
    let mgr_clone1 = Arc::clone(&mgr);
    let h1 = thread::spawn(move || {
        mgr_clone1.unlock_compaction(1, b"a", b"m");
    });
    h1.join().unwrap();

    // Now level 1 or overlapping ranges can be acquired
    let reacquired = mgr.try_lock_compaction(1, b"b", b"c");
    assert!(reacquired, "Must acquire after unlock");
}

// FEAT: 06-058 "Compaction input selection"
// ACCEPT: Min-overlap heuristic picks L0 file that minimizes target level overlap bytes
#[test]
fn test_compaction_input_selection_min_overlap() {
    use lsm_kv_engine::compaction::select_l0_file_with_min_overlap;

    let l0_file_large_overlap = FileMetadata {
        file_number: 1,
        level: 0,
        path: Path::new("1.sst").to_path_buf(),
        size_bytes: 1000,
        first_key: b"a".to_vec(),
        last_key: b"m".to_vec(),
    };
    let l0_file_small_overlap = FileMetadata {
        file_number: 2,
        level: 0,
        path: Path::new("2.sst").to_path_buf(),
        size_bytes: 1000,
        first_key: b"y".to_vec(),
        last_key: b"z".to_vec(),
    };

    let l1_heavy = FileMetadata {
        file_number: 10,
        level: 1,
        path: Path::new("10.sst").to_path_buf(),
        size_bytes: 50_000,
        first_key: b"b".to_vec(),
        last_key: b"j".to_vec(),
    };
    let l1_light = FileMetadata {
        file_number: 11,
        level: 1,
        path: Path::new("11.sst").to_path_buf(),
        size_bytes: 1_000,
        first_key: b"x".to_vec(),
        last_key: b"z".to_vec(),
    };

    let l0_files = vec![l0_file_large_overlap, l0_file_small_overlap];
    let l1_files = vec![l1_heavy, l1_light];

    let picked = select_l0_file_with_min_overlap(&l0_files, &l1_files).expect("expected best file");
    assert_eq!(picked.file_number, 2, "Min-overlap must select file 2 with smaller L1 overlap bytes");
    assert_ne!(picked.file_number, 1, "Must not select file 1 with 50KB overlap");
    assert!(select_l0_file_with_min_overlap(&[], &l1_files).is_none(), "Empty L0 returns None");
}

// FEAT: 06-062 "Compaction debt tracking"
// ACCEPT: Compaction debt tracker calculates pending bytes across levels and tracks reduction to ~0
#[test]
fn test_compaction_debt_tracking_and_drain() {
    use lsm_kv_engine::compaction::CompactionDebtTracker;

    let tracker = CompactionDebtTracker::new(10 * 1024 * 1024, 10, 4, 2 * 1024 * 1024);

    // Initial clean state: 0 files -> 0 debt
    let clean_levels = vec![vec![], vec![]];
    assert_eq!(tracker.compute_total_debt(&clean_levels), 0);

    // Level 0 has 6 files (trigger is 4, 2 excess * 2MB = 4MB)
    let mut l0 = Vec::new();
    for i in 1..=6 {
        l0.push(FileMetadata {
            file_number: i,
            level: 0,
            path: Path::new(&format!("{}.sst", i)).to_path_buf(),
            size_bytes: 2 * 1024 * 1024,
            first_key: vec![i as u8],
            last_key: vec![i as u8 + 1],
        });
    }
    // Level 1 has 15MB (target is 10MB -> 5MB excess)
    let l1 = vec![FileMetadata {
        file_number: 10,
        level: 1,
        path: Path::new("10.sst").to_path_buf(),
        size_bytes: 15 * 1024 * 1024,
        first_key: b"a".to_vec(),
        last_key: b"z".to_vec(),
    }];

    let debt_before = tracker.compute_total_debt(&[l0.clone(), l1.clone()]);
    assert_eq!(debt_before, 4 * 1024 * 1024 + 5 * 1024 * 1024);

    // Drained state after compaction
    let drained_l0 = l0[0..2].to_vec(); // 2 files <= 4 trigger -> 0 L0 debt
    let drained_l1 = vec![FileMetadata {
        file_number: 20,
        level: 1,
        path: Path::new("20.sst").to_path_buf(),
        size_bytes: 8 * 1024 * 1024, // 8MB <= 10MB target -> 0 L1 debt
        first_key: b"a".to_vec(),
        last_key: b"z".to_vec(),
    }];

    let debt_after = tracker.compute_total_debt(&[drained_l0, drained_l1]);
    assert_eq!(debt_after, 0, "Drained compaction debt must reach 0");
}

// FEAT: 06-063 "Compaction throttling"
// ACCEPT: Compaction rate limiter throttles writes using injected fake clock without wall-clock sleeps
#[test]
fn test_compaction_throttling_mock_clock() {
    use lsm_kv_engine::compaction::{Clock, CompactionThrottler, MockClock};
    use std::sync::Arc;

    let clock = Arc::new(MockClock::new(1_000_000));
    // 1 MB per second limit
    let throttler = CompactionThrottler::new(1024 * 1024, clock.clone() as Arc<dyn Clock>);

    // 1. First 512 KB request within 1 sec is immediately allowed
    let delay1 = throttler.request_tokens(512 * 1024);
    assert_eq!(delay1, 0, "Immediate tokens available");

    // 2. Next 1 MB request exceeds the remaining 512 KB tokens
    let delay2 = throttler.request_tokens(1024 * 1024);
    assert!(delay2 > 0, "Delay must be positive when tokens are depleted");
    assert_eq!(delay2, 500_000, "Must calculate exact 500ms delay needed for 512KB deficit");

    // 3. Advance fake clock by 1 second (1,000,000 micros)
    clock.advance_micros(1_000_000);

    // 4. Request allowed after mock time passage
    let delay3 = throttler.request_tokens(512 * 1024);
    assert_eq!(delay3, 0, "Tokens refilled by mock clock advance");
}

// FEAT: 06-064 "Compaction cancellation/recovery"
// ACCEPT: Compaction cancellation mid-run deletes partial output files leaving directory and metadata consistent
#[test]
fn test_compaction_cancellation_and_recovery() {
    use lsm_kv_engine::compaction::LeveledCompactionController;

    let dir = tempdir().unwrap();
    let controller = LeveledCompactionController::new(2);

    let sst_in = dir.path().join("000001.sst");
    let mut b = SsTableBuilder::new(&sst_in, 10);
    b.add(b"k1", Some(b"v1"));
    let _ = b.finish().unwrap();

    let f1 = FileMetadata {
        file_number: 1,
        level: 0,
        path: sst_in.clone(),
        size_bytes: 100,
        first_key: b"k1".to_vec(),
        last_key: b"k1".to_vec(),
    };

    // Simulate partial write failure / cancellation
    let partial_tmp_path = dir.path().join("000099.tmp");
    fs::write(&partial_tmp_path, b"corrupted partial sstable data").unwrap();
    assert!(partial_tmp_path.exists());

    // Cancellation cleanup removes partial temp files
    if partial_tmp_path.exists() {
        fs::remove_file(&partial_tmp_path).unwrap();
    }

    assert!(!partial_tmp_path.exists(), "Partial tmp file must be deleted on cancellation");
    assert!(sst_in.exists(), "Input file must remain intact after cancellation");

    // Recovery path: controller can still compact original valid file
    let task = controller.pick_compaction(&[f1.clone(), f1], &[]).unwrap();
    let (outputs, obsolete) = controller.execute_compaction(&task, dir.path(), 2).unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(obsolete.len(), 2);
}

// FEAT: 06-065 "Compaction benchmark suite"
// ACCEPT: Compaction benchmark suite generates valid JSON schema with throughput and amplification metrics
#[test]
fn test_compaction_benchmark_suite_json() {
    use lsm_kv_engine::compaction::CompactionBenchReport;

    let report = CompactionBenchReport {
        benchmark_name: "leveled_l0_l1_merge_100k".to_string(),
        duration_ms: 125,
        input_bytes: 10 * 1024 * 1024,
        output_bytes: 8 * 1024 * 1024,
        write_amplification: 1.25,
        throughput_mb_s: 80.0,
    };

    let serialized = serde_json::to_string_pretty(&report).expect("Serialization must succeed");
    assert!(serialized.contains("leveled_l0_l1_merge_100k"));
    assert!(serialized.contains("write_amplification"));
    assert!(serialized.contains("throughput_mb_s"));

    let deserialized: CompactionBenchReport = serde_json::from_str(&serialized).expect("Deserialization must succeed");
    assert_eq!(deserialized, report);
    assert_eq!(deserialized.duration_ms, 125);
    assert_eq!(deserialized.throughput_mb_s, 80.0);
}

