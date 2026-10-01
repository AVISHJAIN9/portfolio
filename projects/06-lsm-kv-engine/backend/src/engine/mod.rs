//! High-Performance LSM Key-Value Engine (06-001..009)

use crate::memtable::{MemTable, ValueEntry};
use crate::sstable::{SsTableBuilder, SsTableReader};
use crate::wal::{WalOpType, WalRecord, WalWriter};
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct LsmEngine {
    data_dir: PathBuf,
    memtable: Arc<RwLock<MemTable>>,
    immutable_memtables: Arc<RwLock<Vec<Arc<MemTable>>>>,
    sstables: Arc<RwLock<Vec<SsTableReader>>>,
    wal: Arc<RwLock<WalWriter>>,
    next_seq_no: AtomicU64,
    memtable_threshold_bytes: usize,
    sstable_counter: AtomicU64,
}

impl LsmEngine {
    pub fn open<P: AsRef<Path>>(data_dir: P) -> std::io::Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(&data_dir)?;

        let wal_path = data_dir.join("current.wal");
        let (recovered_records, max_seq) = WalWriter::recover(&wal_path)?;

        let memtable = MemTable::new();
        for rec in &recovered_records {
            match rec.op_type {
                WalOpType::Put => {
                    if let Some(ref val) = rec.value {
                        memtable.put(&rec.key, val);
                    }
                }
                WalOpType::Delete => {
                    memtable.delete(&rec.key);
                }
            }
        }

        let wal = WalWriter::open(&wal_path)?;

        // Scan existing SSTables
        let mut sstables = Vec::new();
        let mut sst_idx = 0u64;
        if let Ok(entries) = fs::read_dir(&data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sst") {
                    if let Ok(reader) = SsTableReader::open(&path) {
                        sstables.push(reader);
                        sst_idx += 1;
                    }
                }
            }
        }

        Ok(Self {
            data_dir,
            memtable: Arc::new(RwLock::new(memtable)),
            immutable_memtables: Arc::new(RwLock::new(Vec::new())),
            sstables: Arc::new(RwLock::new(sstables)),
            wal: Arc::new(RwLock::new(wal)),
            next_seq_no: AtomicU64::new(max_seq + 1),
            memtable_threshold_bytes: 64 * 1024, // 64 KB threshold for flushing
            sstable_counter: AtomicU64::new(sst_idx),
        })
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> std::io::Result<()> {
        let seq = self.next_seq_no.fetch_add(1, Ordering::SeqCst);
        let record = WalRecord {
            seq_no: seq,
            op_type: WalOpType::Put,
            key: key.to_vec(),
            value: Some(value.to_vec()),
        };

        {
            let mut wal = self.wal.write();
            wal.append(&record, true)?;
        }

        {
            let mem = self.memtable.read();
            mem.put(key, value);
        }

        self.maybe_freeze_and_flush()?;
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
        // 1. Check active memtable
        {
            let mem = self.memtable.read();
            if let Some(entry) = mem.get(key) {
                return match entry {
                    ValueEntry::Value(v) => Ok(Some(v)),
                    ValueEntry::Tombstone => Ok(None),
                };
            }
        }

        // 2. Check immutable memtables (newest to oldest)
        {
            let imm_list = self.immutable_memtables.read();
            for imm in imm_list.iter().rev() {
                if let Some(entry) = imm.get(key) {
                    return match entry {
                        ValueEntry::Value(v) => Ok(Some(v)),
                        ValueEntry::Tombstone => Ok(None),
                    };
                }
            }
        }

        // 3. Check SSTables (newest to oldest)
        {
            let sst_list = self.sstables.read();
            for sst in sst_list.iter().rev() {
                if let Some(val_opt) = sst.get(key)? {
                    return Ok(val_opt);
                }
            }
        }

        Ok(None)
    }

    pub fn delete(&self, key: &[u8]) -> std::io::Result<()> {
        let seq = self.next_seq_no.fetch_add(1, Ordering::SeqCst);
        let record = WalRecord {
            seq_no: seq,
            op_type: WalOpType::Delete,
            key: key.to_vec(),
            value: None,
        };

        {
            let mut wal = self.wal.write();
            wal.append(&record, true)?;
        }

        {
            let mem = self.memtable.read();
            mem.delete(key);
        }

        self.maybe_freeze_and_flush()?;
        Ok(())
    }

    pub fn write_batch(&self, puts: &[(&[u8], &[u8])], deletes: &[&[u8]]) -> std::io::Result<()> {
        for &(k, v) in puts {
            self.put(k, v)?;
        }
        for &k in deletes {
            self.delete(k)?;
        }
        Ok(())
    }

    pub fn scan_range(&self, start_key: &[u8], end_key: &[u8]) -> std::io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut merged: BTreeMap<Vec<u8>, Option<Vec<u8>>> = BTreeMap::new();

        // 1. SSTables
        {
            let sst_list = self.sstables.read();
            for sst in sst_list.iter() {
                for (k, v) in sst.scan_all()? {
                    if k.as_slice() >= start_key && k.as_slice() <= end_key {
                        merged.insert(k, v);
                    }
                }
            }
        }

        // 2. Immutable memtables
        {
            let imm_list = self.immutable_memtables.read();
            for imm in imm_list.iter() {
                for (k, entry) in imm.iter() {
                    if k.as_slice() >= start_key && k.as_slice() <= end_key {
                        match entry {
                            ValueEntry::Value(v) => merged.insert(k, Some(v)),
                            ValueEntry::Tombstone => merged.insert(k, None),
                        };
                    }
                }
            }
        }

        // 3. Active memtable
        {
            let mem = self.memtable.read();
            for (k, entry) in mem.iter() {
                if k.as_slice() >= start_key && k.as_slice() <= end_key {
                    match entry {
                        ValueEntry::Value(v) => merged.insert(k, Some(v)),
                        ValueEntry::Tombstone => merged.insert(k, None),
                    };
                }
            }
        }

        let mut results = Vec::new();
        for (k, v) in merged {
            if let Some(val) = v {
                results.push((k, val));
            }
        }
        Ok(results)
    }

    pub fn scan_prefix(&self, prefix: &[u8]) -> std::io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut end_key = prefix.to_vec();
        for b in end_key.iter_mut().rev() {
            if *b < 0xff {
                *b += 1;
                break;
            }
        }
        self.scan_range(prefix, &end_key)
    }

    pub fn flush(&self) -> std::io::Result<()> {
        let entries = {
            let mut mem = self.memtable.write();
            let entries = mem.iter();
            *mem = MemTable::new();
            entries
        };

        if entries.is_empty() {
            return Ok(());
        }

        let sst_id = self.sstable_counter.fetch_add(1, Ordering::SeqCst);
        let sst_path = self.data_dir.join(format!("{:06}.sst", sst_id));

        let mut builder = SsTableBuilder::new(&sst_path, entries.len());
        for (k, entry) in entries {
            match entry {
                ValueEntry::Value(v) => builder.add(&k, Some(&v)),
                ValueEntry::Tombstone => builder.add(&k, None),
            }
        }

        let reader = builder.finish()?;
        self.sstables.write().push(reader);
        Ok(())
    }

    fn maybe_freeze_and_flush(&self) -> std::io::Result<()> {
        let size = self.memtable.read().size_bytes();
        if size >= self.memtable_threshold_bytes {
            self.flush()?;
        }
        Ok(())
    }
}
