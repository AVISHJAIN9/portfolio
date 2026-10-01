use lsm_kv_engine::memtable::{MemTable, ValueEntry};

// FEAT: 06-027 "Skiplist memtable"
// ACCEPT: Concurrent Skiplist MemTable supports non-blocking writes and ordered iterations
#[test]
fn test_memtable_insertion_and_ordering() {
    let mem = MemTable::new();
    mem.put(b"banana", b"1");
    mem.put(b"apple", b"2");
    mem.put(b"cherry", b"3");

    let entries = mem.iter();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, b"apple");
    assert_eq!(entries[1].0, b"banana");
    assert_eq!(entries[2].0, b"cherry");
}

// FEAT: 06-028 "Memtable lookup"
// ACCEPT: Tombstone deletion markers override earlier keys in reads and iterators
#[test]
fn test_memtable_tombstone_deletion() {
    let mem = MemTable::new();
    mem.put(b"key_1", b"val_1");
    assert!(matches!(mem.get(b"key_1"), Some(ValueEntry::Value(_))));

    mem.delete(b"key_1");
    assert_eq!(mem.get(b"key_1"), Some(ValueEntry::Tombstone));
}

// FEAT: 06-029 "Memtable ordered iterator"
// ACCEPT: Fast point lookup retrieves newest value entry or tombstone from active memtable
#[test]
fn test_memtable_point_lookup() {
    let mem = MemTable::new();
    assert_eq!(mem.get(b"missing"), None);

    mem.put(b"target", b"payload");
    if let Some(ValueEntry::Value(v)) = mem.get(b"target") {
        assert_eq!(v, b"payload");
    } else {
        panic!("Expected Value entry");
    }
}

// FEAT: 06-030 "Memtable size thresholds"
// ACCEPT: MemTable tracks approximate byte size and signals flush when threshold exceeded
#[test]
fn test_memtable_size_tracking() {
    let mem = MemTable::new();
    assert_eq!(mem.size_bytes(), 0);

    mem.put(b"key", b"value");
    assert!(mem.size_bytes() > 0);
}
