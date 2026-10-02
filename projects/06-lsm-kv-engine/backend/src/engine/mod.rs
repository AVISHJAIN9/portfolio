//! High-Performance LSM Key-Value Engine (06-001..009)

pub mod admin;
pub mod txn;

pub use admin::*;
pub use txn::*;

use crate::memtable::{MemTable, ValueEntry};
use crate::sstable::{SsTableBuilder, SsTableReader};
use crate::wal::{WalOpType, WalRecord, WalWriter};
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};
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
    pub limits: EngineLimits,
    pub metrics: Arc<EngineMetrics>,
    key_modifications: Arc<RwLock<HashMap<Vec<u8>, u64>>>,
}

impl LsmEngine {
    pub fn open<P: AsRef<Path>>(data_dir: P) -> std::io::Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(&data_dir)?;

        let wal_path = data_dir.join("current.wal");
        let (recovered_records, max_seq) = WalWriter::recover(&wal_path)?;

        let memtable = MemTable::new();
        let mut key_mods = HashMap::new();
        for rec in &recovered_records {
            key_mods.insert(rec.key.clone(), rec.seq_no);
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
            limits: EngineLimits::default(),
            metrics: Arc::new(EngineMetrics::default()),
            key_modifications: Arc::new(RwLock::new(key_mods)),
        })
    }

    pub fn begin_transaction(&self) -> Transaction<'_> {
        Transaction::new(self)
    }

    pub fn is_key_modified_since(&self, key: &[u8], since_seq: u64) -> bool {
        let mods = self.key_modifications.read();
        if let Some(&last_mod_seq) = mods.get(key) {
            last_mod_seq >= since_seq
        } else {
            false
        }
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> std::io::Result<()> {
        if let Err(e) = self.limits.validate_put(key, value) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{:?}", e),
            ));
        }

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

        self.key_modifications.write().insert(key.to_vec(), seq);
        self.metrics.put_count.fetch_add(1, Ordering::Relaxed);
        self.maybe_freeze_and_flush()?;
        Ok(())
    }

    pub fn put_with_ttl(&self, key: &[u8], value: &[u8], _ttl_secs: u64) -> std::io::Result<()> {
        // Embed expiration in value or write as standard put
        self.put(key, value)
    }

    pub fn get(&self, key: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
        if let Err(e) = self.limits.validate_key(key) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{:?}", e),
            ));
        }

        self.metrics.get_count.fetch_add(1, Ordering::Relaxed);

        // 1. Check active memtable
        {
            let mem = self.memtable.read();
            if let Some(entry) = mem.get(key) {
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
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
                    self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
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
                    self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(val_opt);
                }
            }
        }

        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        Ok(None)
    }

    pub fn snapshot(&self) -> u64 {
        self.next_seq_no.load(Ordering::SeqCst)
    }

    pub fn batch_get(&self, keys: &[&[u8]]) -> std::io::Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for &k in keys {
            results.push(self.get(k)?);
        }
        Ok(results)
    }

    pub fn delete(&self, key: &[u8]) -> std::io::Result<()> {
        if let Err(e) = self.limits.validate_key(key) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{:?}", e),
            ));
        }

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

        self.key_modifications.write().insert(key.to_vec(), seq);
        self.metrics.delete_count.fetch_add(1, Ordering::Relaxed);
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

    pub fn scan_range(
        &self,
        start_key: &[u8],
        end_key: &[u8],
    ) -> std::io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
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

    pub fn health(&self) -> EngineHealthReport {
        let mem_bytes = self.memtable.read().size_bytes();
        let sst_count = self.sstables.read().len();
        let wal_size = fs::metadata(self.data_dir.join("current.wal"))
            .map(|m| m.len())
            .unwrap_or(0);

        let status = if mem_bytes > 128 * 1024 * 1024 {
            HealthStatus::Degraded("Memtable backpressure stall".to_string())
        } else {
            HealthStatus::Ok
        };

        EngineHealthReport {
            status,
            active_levels: sst_count,
            memtable_bytes: mem_bytes,
            pending_compaction_bytes: 0,
            wal_size_bytes: wal_size,
            is_stalled: false,
        }
    }

    fn maybe_freeze_and_flush(&self) -> std::io::Result<()> {
        let size = self.memtable.read().size_bytes();
        if size >= self.memtable_threshold_bytes {
            self.flush()?;
        }
        Ok(())
    }
}
