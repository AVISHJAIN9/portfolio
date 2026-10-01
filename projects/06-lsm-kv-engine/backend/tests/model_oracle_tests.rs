use lsm_kv_engine::engine::LsmEngine;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_lsm_engine_model_oracle_differential() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    let mut oracle: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();

    let mut rng = StdRng::seed_from_u64(42);

    for op_i in 0..1000 {
        let op_type = rng.gen_range(0..4);
        let key_idx = rng.gen_range(0..100);
        let key = format!("k_{:04}", key_idx).into_bytes();

        match op_type {
            0 | 1 => {
                // Put
                let val = format!("val_{}_{}", key_idx, op_i).into_bytes();
                engine.put(&key, &val).unwrap();
                oracle.insert(key, val);
            }
            2 => {
                // Delete
                engine.delete(&key).unwrap();
                oracle.remove(&key);
            }
            3 => {
                // Get & Assert
                let eng_val = engine.get(&key).unwrap();
                let oracle_val = oracle.get(&key).cloned();
                assert_eq!(eng_val, oracle_val, "Mismatch on key {:?}", String::from_utf8_lossy(&key));
            }
            _ => {}
        }

        if op_i % 200 == 0 {
            engine.flush().unwrap();
        }
    }

    // Final full scan comparison
    let all_eng = engine.scan_range(b"k_0000", b"k_9999").unwrap();
    let all_oracle: Vec<_> = oracle.into_iter().collect();
    assert_eq!(all_eng, all_oracle, "Full range scan mismatch against BTreeMap oracle");
}

#[test]
fn test_lsm_crash_recovery_injected_kill() {
    let dir = tempdir().unwrap();
    let db_path = dir.path();

    // 1. Initial writes
    {
        let engine = LsmEngine::open(db_path).unwrap();
        for i in 1..=50 {
            engine.put(format!("key_{:03}", i).as_bytes(), format!("val_{:03}", i).as_bytes()).unwrap();
        }
    }

    // 2. Inject corrupted tail bytes into WAL
    let wal_path = db_path.join("current.wal");
    {
        let mut file = OpenOptions::new().append(true).open(&wal_path).unwrap();
        file.write_all(b"PARTIAL_UNFLUSHED_CORRUPTED_BYTES_FROM_KILL").unwrap();
    }

    // 3. Reopen engine and assert recovered prefix state
    {
        let reopened = LsmEngine::open(db_path).unwrap();
        for i in 1..=50 {
            let key = format!("key_{:03}", i).into_bytes();
            let val = reopened.get(&key).unwrap();
            assert_eq!(val, Some(format!("val_{:03}", i).into_bytes()));
        }
    }
}
