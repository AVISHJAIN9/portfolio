//! WAL & Durability (Features 08-040..08-052)

use crate::model::Record;
use bytes::Buf;
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsyncPolicy {
    Always,
    GroupCommit { batch_size: usize, timeout_ms: u64 },
    OsDefault,
}

/// 08-040..08-052: Write Ahead Log Manager
pub struct WriteAheadLog {
    file_path: PathBuf,
    file: Mutex<File>,
    policy: FsyncPolicy,
    flushed_offset: AtomicU64,
    unflushed_count: Mutex<usize>,
}

impl WriteAheadLog {
    pub fn open(path: impl AsRef<Path>, policy: FsyncPolicy) -> std::io::Result<Self> {
        let file_path = path.as_ref().to_path_buf();
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&file_path)?;

        Ok(Self {
            file_path,
            file: Mutex::new(file),
            policy,
            flushed_offset: AtomicU64::new(0),
            unflushed_count: Mutex::new(0),
        })
    }

    /// 08-041..08-045: Append WAL entry and execute fsync according to policy
    pub fn write_entry(&self, record: &Record) -> std::io::Result<()> {
        let encoded = record.encode();
        let mut file = self.file.lock();
        file.write_all(&encoded)?;

        match &self.policy {
            FsyncPolicy::Always => {
                file.sync_data()?;
                self.flushed_offset.store(record.offset, Ordering::SeqCst);
            }
            FsyncPolicy::GroupCommit { batch_size, .. } => {
                let mut count = self.unflushed_count.lock();
                *count += 1;
                if *count >= *batch_size {
                    file.sync_data()?;
                    *count = 0;
                    self.flushed_offset.store(record.offset, Ordering::SeqCst);
                }
            }
            FsyncPolicy::OsDefault => {
                // Let OS buffer manage flushing
                self.flushed_offset.store(record.offset, Ordering::SeqCst);
            }
        }
        Ok(())
    }

    /// 08-045: Explicit flush for group commit timeout
    pub fn flush(&self) -> std::io::Result<()> {
        let file = self.file.lock();
        file.sync_data()?;
        let mut count = self.unflushed_count.lock();
        *count = 0;
        Ok(())
    }

    /// 08-046..08-048: Replay WAL and recover valid records, truncating partial writes
    pub fn replay_and_recover(&self) -> std::io::Result<(Vec<Record>, usize)> {
        let mut file = OpenOptions::new().read(true).write(true).open(&self.file_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let mut slice = bytes::Bytes::from(data.clone());
        let mut records = Vec::new();
        let mut valid_bytes = 0;
        let mut corrupted_bytes = 0;

        while slice.remaining() >= 20 {
            let current = slice.clone();
            match Record::decode(current) {
                Ok(rec) => {
                    let len = rec.encode().len();
                    slice.advance(len);
                    valid_bytes += len;
                    records.push(rec);
                }
                Err(_) => {
                    // Corrupted or partial write encountered at tail
                    corrupted_bytes = slice.remaining();
                    slice.clear();
                    break;
                }
            }
        }

        if slice.remaining() > 0 {
            corrupted_bytes += slice.remaining();
        }

        if corrupted_bytes > 0 {
            // 08-048: Truncate to last valid boundary
            file.set_len(valid_bytes as u64)?;
        }

        Ok((records, corrupted_bytes))
    }

    pub fn flushed_offset(&self) -> u64 {
        self.flushed_offset.load(Ordering::SeqCst)
    }
}
