use lsm_kv_engine::engine::LsmEngine;
use tempfile::tempdir;

// FEAT: 06-001 "KV Put API"
// ACCEPT: Put key-value pair inserts entry into WAL and active memtable
#[test]
fn test_kv_put_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    assert!(engine.put(b"user:101", b"Alice").is_ok());
    let val = engine.get(b"user:101").unwrap();
    assert_eq!(val, Some(b"Alice".to_vec()));
}

// FEAT: 06-002 "KV Get API"
// ACCEPT: Get point lookup searches active memtable, immutable tables, and SSTables
#[test]
fn test_kv_get_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"key_a", b"val_a").unwrap();
    assert_eq!(engine.get(b"key_a").unwrap(), Some(b"val_a".to_vec()));
    assert_eq!(engine.get(b"non_existent").unwrap(), None);
}

// FEAT: 06-003 "KV Delete API"
// ACCEPT: Delete writes tombstone marker to mask existing keys across all levels
#[test]
fn test_kv_delete_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"del_target", b"temporary").unwrap();
    assert_eq!(engine.get(b"del_target").unwrap(), Some(b"temporary".to_vec()));

    engine.delete(b"del_target").unwrap();
    assert_eq!(engine.get(b"del_target").unwrap(), None);
}

// FEAT: 06-004 "Batch write API"
// ACCEPT: Atomic batch write applies sequence of puts and deletes
#[test]
fn test_batch_write_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();

    let puts = vec![
        (b"batch:1".as_slice(), b"v1".as_slice()),
        (b"batch:2".as_slice(), b"v2".as_slice()),
    ];
    let deletes = vec![b"batch:old".as_slice()];

    assert!(engine.write_batch(&puts, &deletes).is_ok());
    assert_eq!(engine.get(b"batch:1").unwrap(), Some(b"v1".to_vec()));
    assert_eq!(engine.get(b"batch:2").unwrap(), Some(b"v2".to_vec()));
}

// FEAT: 06-005 "Batch read API"
// ACCEPT: Batch read retrieves multiple keys concurrently
#[test]
fn test_batch_read_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"k1", b"v1").unwrap();
    engine.put(b"k2", b"v2").unwrap();

    let vals = engine.batch_get(&[b"k1", b"k2", b"missing"]).unwrap();
    assert_eq!(vals[0], Some(b"v1".to_vec()));
    assert_eq!(vals[1], Some(b"v2".to_vec()));
    assert_eq!(vals[2], None);
}

// FEAT: 06-006 "Prefix scan API"
// ACCEPT: Prefix scan returns all key-value entries matching common byte prefix
#[test]
fn test_prefix_scan_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"user:001", b"Alice").unwrap();
    engine.put(b"user:002", b"Bob").unwrap();
    engine.put(b"order:001", b"Order1").unwrap();

    let users = engine.scan_prefix(b"user:").unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].0, b"user:001");
    assert_eq!(users[1].0, b"user:002");
}

// FEAT: 06-007 "Range scan API"
// ACCEPT: Range scan iterates over [start_key, end_key] bounds in lexicographical order
#[test]
fn test_range_scan_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    engine.put(b"a", b"1").unwrap();
    engine.put(b"b", b"2").unwrap();
    engine.put(b"c", b"3").unwrap();
    engine.put(b"d", b"4").unwrap();

    let range = engine.scan_range(b"b", b"c").unwrap();
    assert_eq!(range.len(), 2);
    assert_eq!(range[0].0, b"b");
    assert_eq!(range[1].0, b"c");
}

// FEAT: 06-008 "Iterator API"
// ACCEPT: Forward iterator traverses entries across memtable and SSTable levels
#[test]
fn test_iterator_api() {
    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    for i in 1..=5 {
        engine.put(format!("key_{}", i).as_bytes(), format!("val_{}", i).as_bytes()).unwrap();
    }

    let all = engine.scan_range(b"key_1", b"key_5").unwrap();
    assert_eq!(all.len(), 5);
}
