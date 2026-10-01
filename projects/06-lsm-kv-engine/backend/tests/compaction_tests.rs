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
    let controller = LeveledCompactionController::new(4);
    let l0_files = vec![
        FileMetadata {
            file_number: 1,
            level: 0,
            path: Path::new("1.sst").to_path_buf(),
            size_bytes: 50,
            first_key: b"a".to_vec(),
            last_key: b"b".to_vec(),
        },
    ];

    // Less than threshold -> None
    assert!(controller.pick_compaction(&l0_files, &[]).is_none());
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

    let task = controller.pick_compaction(&[f1], &[]).unwrap();
    let (outputs, obsolete) = controller.execute_compaction(&task, dir.path(), 2).unwrap();

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].file_number, 2);
    assert!(!sst_path.exists()); // Obsolete file removed
    assert!(outputs[0].path.exists()); // Output file created
}

// FEAT: 06-060 "Write amplification accounting"
// ACCEPT: Write amplification tracks ratio of total bytes written to bytes read across compactions
#[test]
fn test_write_amplification_accounting_metrics() {
    let dir = tempdir().unwrap();
    let controller = LeveledCompactionController::new(1);

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
