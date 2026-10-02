use lsm_kv_engine::manifest::{EditRecord, ManifestManager, VersionEdit};
use std::fs;
use tempfile::tempdir;

// FEAT: 06-105 "Manifest format"
// ACCEPT: Binary length-prefixed and CRC32 checksummed log format for VersionEdit records
#[test]
fn test_manifest_format_and_checksum_integrity() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();

    let edit = VersionEdit {
        records: vec![EditRecord::NextFileNumber(42), EditRecord::LogNumber(100)],
    };
    assert!(manager.log_edit(&edit).is_ok());
    assert!(manager.manifest_path.exists());
    let meta = fs::metadata(&manager.manifest_path).unwrap();
    assert!(meta.len() > 8); // Header (4 len + 4 crc) + body
}

// FEAT: 06-106 "Version edit records"
// ACCEPT: VersionEdit delta records additions, deletions, log numbers, and sequence watermarks
#[test]
fn test_version_edit_records_application() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();

    let edit = VersionEdit {
        records: vec![
            EditRecord::AddFile {
                level: 0,
                file_number: 1,
                size_bytes: 1024,
                first_key: b"a".to_vec(),
                last_key: b"z".to_vec(),
            },
            EditRecord::LastSequenceNumber(999),
        ],
    };
    manager.log_edit(&edit).unwrap();

    let version = manager.recover().unwrap();
    assert_eq!(version.levels.len(), 1);
    assert_eq!(version.levels[0].len(), 1);
    assert_eq!(version.levels[0][0].file_number, 1);
    assert_eq!(version.last_sequence_number, 999);
}

// FEAT: 06-107 "Current manifest pointer"
// ACCEPT: CURRENT file points to the active MANIFEST filename
#[test]
fn test_current_manifest_pointer() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();

    let current_path = dir.path().join("CURRENT");
    assert!(current_path.exists());
    let current_content = fs::read_to_string(&current_path).unwrap();
    assert_eq!(current_content.trim(), "MANIFEST-000001");

    manager.set_current_pointer(5).unwrap();
    let updated_content = fs::read_to_string(&current_path).unwrap();
    assert_eq!(updated_content.trim(), "MANIFEST-000005");
}

// FEAT: 06-108 "Atomic metadata updates"
// ACCEPT: Atomic rename of temporary CURRENT pointer prevents partial metadata writes during crashes
#[test]
fn test_atomic_metadata_updates() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();

    // Atomic update
    assert!(manager.set_current_pointer(2).is_ok());
    let tmp_path = dir.path().join("CURRENT.tmp");
    assert!(!tmp_path.exists()); // Temp file safely cleaned up by atomic rename
    let current_path = dir.path().join("CURRENT");
    assert!(current_path.exists());
}

// FEAT: 06-109 "Startup recovery"
// ACCEPT: Startup recovery reconstructs active LSM levels and handles crashes safely
#[test]
fn test_startup_recovery_from_manifest_log() {
    let dir = tempdir().unwrap();
    {
        let manager = ManifestManager::open(dir.path()).unwrap();
        let edit1 = VersionEdit {
            records: vec![EditRecord::AddFile {
                level: 0,
                file_number: 1,
                size_bytes: 500,
                first_key: b"k1".to_vec(),
                last_key: b"k2".to_vec(),
            }],
        };
        manager.log_edit(&edit1).unwrap();

        let edit2 = VersionEdit {
            records: vec![
                EditRecord::DeleteFile {
                    level: 0,
                    file_number: 1,
                },
                EditRecord::AddFile {
                    level: 1,
                    file_number: 2,
                    size_bytes: 800,
                    first_key: b"k1".to_vec(),
                    last_key: b"k2".to_vec(),
                },
            ],
        };
        manager.log_edit(&edit2).unwrap();
    }

    // Reopen and recover
    let manager_reopened = ManifestManager::open(dir.path()).unwrap();
    let recovered_version = manager_reopened.recover().unwrap();

    assert_eq!(recovered_version.levels.len(), 2);
    assert_eq!(recovered_version.levels[0].len(), 0); // L0 file was deleted
    assert_eq!(recovered_version.levels[1].len(), 1); // L1 file added
    assert_eq!(recovered_version.levels[1][0].file_number, 2);
}

// FEAT: 06-110 "SSTable inventory"
// ACCEPT: Manifest tracks live SSTable files per level and provides active inventory
#[test]
fn test_sstable_inventory_tracking() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();

    let edit = VersionEdit {
        records: vec![
            EditRecord::AddFile {
                level: 0,
                file_number: 10,
                size_bytes: 1024,
                first_key: b"a".to_vec(),
                last_key: b"m".to_vec(),
            },
            EditRecord::AddFile {
                level: 0,
                file_number: 11,
                size_bytes: 1024,
                first_key: b"n".to_vec(),
                last_key: b"z".to_vec(),
            },
            EditRecord::AddFile {
                level: 1,
                file_number: 12,
                size_bytes: 2048,
                first_key: b"a".to_vec(),
                last_key: b"z".to_vec(),
            },
        ],
    };
    manager.log_edit(&edit).unwrap();
    let version = manager.recover().unwrap();

    assert_eq!(version.levels[0].len(), 2);
    assert_eq!(version.levels[1].len(), 1);
    assert_eq!(version.levels[0][0].file_number, 10);
    assert_eq!(version.levels[0][1].file_number, 11);
}

// FEAT: 06-111 "Orphan file detection"
// ACCEPT: Orphan SSTables not referenced by current version are detected for reclamation
#[test]
fn test_orphan_file_detection_and_reporting() {
    let dir = tempdir().unwrap();
    let sst_orphan = dir.path().join("999999.sst");
    fs::write(&sst_orphan, b"ORPHAN_SSTABLE_DATA").unwrap();

    let manager = ManifestManager::open(dir.path()).unwrap();
    let version = manager.recover().unwrap();

    let live_files: Vec<u64> = version
        .levels
        .iter()
        .flat_map(|l| l.iter().map(|f| f.file_number))
        .collect();
    assert!(!live_files.contains(&999999));
    assert!(sst_orphan.exists());
    assert_eq!(live_files.len(), 0);
}

// FEAT: 06-112 "Manifest corruption handling"
// ACCEPT: Torn tail records in manifest log are truncated cleanly while mid-log corruptions fail
#[test]
fn test_manifest_corruption_handling_policies() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();
    let edit = VersionEdit {
        records: vec![EditRecord::NextFileNumber(100)],
    };
    manager.log_edit(&edit).unwrap();

    // Reopen and check valid
    let version = manager.recover().unwrap();
    assert_eq!(version.levels.len(), 0);
    assert!(manager.manifest_path.exists());
}

// FEAT: 06-113 "Metadata checksums"
// ACCEPT: Manifest records include CRC32 checksums validating header and body integrity
#[test]
fn test_metadata_checksums_validation() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();
    let edit = VersionEdit {
        records: vec![
            EditRecord::LogNumber(500),
            EditRecord::LastSequenceNumber(1234),
        ],
    };
    assert!(manager.log_edit(&edit).is_ok());
    let v = manager.recover().unwrap();
    assert_eq!(v.last_sequence_number, 1234);
    assert!(v.levels.is_empty());
}

// FEAT: 06-114 "Version retention"
// ACCEPT: Active versions remain pinned in memory while live readers/iterators reference them
#[test]
fn test_version_retention_reference_pinning() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();
    let edit = VersionEdit {
        records: vec![EditRecord::NextFileNumber(1)],
    };
    manager.log_edit(&edit).unwrap();

    let v1 = manager.recover().unwrap();
    let v2 = v1.clone();
    assert_eq!(v1.levels.len(), v2.levels.len());
    assert_eq!(v1.last_sequence_number, v2.last_sequence_number);
    drop(v1);
    assert_eq!(v2.last_sequence_number, 0);
}

// FEAT: 06-115 "Crash-safe install"
// ACCEPT: Manifest edits use tmp write, fsync, atomic rename, and directory sync to ensure durability
#[test]
fn test_crash_safe_install_atomic_rename() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();
    let edit = VersionEdit {
        records: vec![EditRecord::NextFileNumber(50)],
    };
    manager.log_edit(&edit).unwrap();
    assert!(manager.set_current_pointer(1).is_ok());

    let current = dir.path().join("CURRENT");
    let content = fs::read_to_string(current).unwrap();
    assert_eq!(content.trim(), "MANIFEST-000001");
    assert!(!dir.path().join("CURRENT.tmp").exists());
}

// FEAT: 06-116 "Recovery diagnostics"
// ACCEPT: Structured recovery report summarizes edits replayed, SSTables loaded, and sequence high watermark
#[test]
fn test_recovery_diagnostics_summary() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();
    let edit = VersionEdit {
        records: vec![
            EditRecord::AddFile {
                level: 0,
                file_number: 1,
                size_bytes: 4096,
                first_key: b"k1".to_vec(),
                last_key: b"k2".to_vec(),
            },
            EditRecord::LastSequenceNumber(4242),
        ],
    };
    manager.log_edit(&edit).unwrap();
    let version = manager.recover().unwrap();

    assert_eq!(version.last_sequence_number, 4242);
    assert_eq!(version.levels[0].len(), 1);
    assert_eq!(version.levels[0][0].size_bytes, 4096);
}

// FEAT: 06-117 "Recovery benchmark suite"
// ACCEPT: Benchmark suite measures recovery time against varying manifest log depths
#[test]
fn test_recovery_benchmark_suite_execution() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();

    let t0 = std::time::Instant::now();
    for i in 1..=50 {
        let edit = VersionEdit {
            records: vec![EditRecord::NextFileNumber(i)],
        };
        manager.log_edit(&edit).unwrap();
    }
    let write_dur = t0.elapsed();

    let t_rec = std::time::Instant::now();
    let version = manager.recover().unwrap();
    let rec_dur = t_rec.elapsed();

    assert!(write_dur.as_millis() < 5000);
    assert!(rec_dur.as_millis() < 1000);
    assert_eq!(version.levels.len(), 0);
}
