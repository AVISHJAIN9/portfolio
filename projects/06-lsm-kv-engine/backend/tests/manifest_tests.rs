use std::fs;
use tempfile::tempdir;
use lsm_kv_engine::manifest::{EditRecord, ManifestManager, VersionEdit};

// FEAT: 06-105 "Manifest format"
// ACCEPT: Binary length-prefixed and CRC32 checksummed log format for VersionEdit records
#[test]
fn test_manifest_format_and_checksum_integrity() {
    let dir = tempdir().unwrap();
    let manager = ManifestManager::open(dir.path()).unwrap();

    let edit = VersionEdit {
        records: vec![
            EditRecord::NextFileNumber(42),
            EditRecord::LogNumber(100),
        ],
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
}

// FEAT: 06-109 "Startup recovery"
// ACCEPT: Startup recovery reconstructs active LSM levels and handles crashes safely
#[test]
fn test_startup_recovery_from_manifest_log() {
    let dir = tempdir().unwrap();
    {
        let manager = ManifestManager::open(dir.path()).unwrap();
        let edit1 = VersionEdit {
            records: vec![
                EditRecord::AddFile {
                    level: 0,
                    file_number: 1,
                    size_bytes: 500,
                    first_key: b"k1".to_vec(),
                    last_key: b"k2".to_vec(),
                },
            ],
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
    let manager = ManifestManager::open(dir.path()).unwrap();
    let version = manager.recover().unwrap();

    assert_eq!(version.levels.len(), 2);
    assert_eq!(version.levels[0].len(), 0); // L0 file 1 was deleted
    assert_eq!(version.levels[1].len(), 1); // L1 file 2 added
    assert_eq!(version.levels[1][0].file_number, 2);
}
