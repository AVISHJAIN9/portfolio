//! Append-Only Log & Segments (Features 08-027..08-039)

use crate::model::Record;
use bytes::Buf;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub offset: u64,
    pub position: u64,
}

pub struct Segment {
    pub base_offset: u64,
    pub max_size_bytes: u64,
    pub current_size_bytes: u64,
    pub index: Vec<IndexEntry>,
    pub file_path: PathBuf,
}

/// 08-027..08-039: Partition Log Manager
pub struct PartitionLog {
    dir: PathBuf,
    segments: RwLock<Vec<Segment>>,
    next_offset: AtomicU64,
    max_segment_size: u64,
    index_interval_records: u64,
}

impl PartitionLog {
    pub fn new(dir: impl AsRef<Path>, max_segment_size: u64, index_interval_records: u64) -> std::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;

        let mut log = Self {
            dir,
            segments: RwLock::new(Vec::new()),
            next_offset: AtomicU64::new(0),
            max_segment_size,
            index_interval_records,
        };
        log.recover_or_init()?;
        Ok(log)
    }

    fn recover_or_init(&mut self) -> std::io::Result<()> {
        let seg_path = self.dir.join("00000000000000000000.log");
        let initial_seg = Segment {
            base_offset: 0,
            max_size_bytes: self.max_segment_size,
            current_size_bytes: 0,
            index: vec![IndexEntry { offset: 0, position: 0 }],
            file_path: seg_path,
        };
        self.segments.write().push(initial_seg);
        Ok(())
    }

    /// 08-027..08-034: Append record to active segment
    pub fn append(&self, key: Option<Vec<u8>>, value: Vec<u8>, headers: HashMap<String, String>, timestamp: i64) -> std::io::Result<Record> {
        let offset = self.next_offset.fetch_add(1, Ordering::SeqCst);
        let record = Record::new(offset, timestamp, key, value, headers);
        let encoded = record.encode();

        let mut segments = self.segments.write();
        let last_idx = segments.len() - 1;
        let active = &mut segments[last_idx];

        // 08-029: Roll segment if limit reached
        if active.current_size_bytes + encoded.len() as u64 > active.max_size_bytes {
            let new_base = offset;
            let new_path = self.dir.join(format!("{:020}.log", new_base));
            let new_seg = Segment {
                base_offset: new_base,
                max_size_bytes: self.max_segment_size,
                current_size_bytes: 0,
                index: vec![IndexEntry { offset: new_base, position: 0 }],
                file_path: new_path,
            };
            segments.push(new_seg);
        }

        let last_idx = segments.len() - 1;
        let active = &mut segments[last_idx];

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active.file_path)?;

        let pos = active.current_size_bytes;
        file.write_all(&encoded)?;
        active.current_size_bytes += encoded.len() as u64;

        // 08-030..08-031: Sparse index lookup entry
        if offset % self.index_interval_records == 0 {
            active.index.push(IndexEntry { offset, position: pos });
        }

        Ok(record)
    }

    /// 08-031: Read records starting from offset using index binary search
    pub fn read_from(&self, start_offset: u64, max_records: usize) -> std::io::Result<Vec<Record>> {
        let segments = self.segments.read();
        let mut results = Vec::new();

        for seg in segments.iter() {
            if seg.base_offset > start_offset && results.len() >= max_records {
                break;
            }
            if !seg.file_path.exists() {
                continue;
            }

            // Find closest position in index via binary search
            let pos = match seg.index.binary_search_by_key(&start_offset, |e| e.offset) {
                Ok(idx) => seg.index[idx].position,
                Err(idx) => {
                    if idx > 0 {
                        seg.index[idx - 1].position
                    } else {
                        0
                    }
                }
            };

            let mut file = File::open(&seg.file_path)?;
            file.seek(SeekFrom::Start(pos))?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            let mut slice = bytes::Bytes::from(buf);
            while slice.remaining() >= 20 && results.len() < max_records {
                let start_slice = slice.clone();
                match Record::decode(start_slice) {
                    Ok(rec) => {
                        let consumed = rec.encode().len();
                        slice.advance(consumed);
                        if rec.offset >= start_offset {
                            results.push(rec);
                        }
                    }
                    Err(_) => break, // Trailing corruption or end of valid stream
                }
            }
        }

        Ok(results)
    }

    /// 08-037: Detect segment corruption
    pub fn verify_integrity(&self) -> std::io::Result<bool> {
        let segments = self.segments.read();
        for seg in segments.iter() {
            if !seg.file_path.exists() {
                continue;
            }
            let mut file = File::open(&seg.file_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            let mut slice = bytes::Bytes::from(buf);
            while slice.remaining() >= 20 {
                let current_slice = slice.clone();
                match Record::decode(current_slice) {
                    Ok(rec) => {
                        let len = rec.encode().len();
                        slice.advance(len);
                    }
                    Err(_) => return Ok(false),
                }
            }
        }
        Ok(true)
    }

    pub fn next_offset(&self) -> u64 {
        self.next_offset.load(Ordering::SeqCst)
    }
}
