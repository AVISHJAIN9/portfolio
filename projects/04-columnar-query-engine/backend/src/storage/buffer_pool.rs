use bytes::Bytes;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub enum StorageError {
    OutOfMemory { requested: usize, available: usize },
    BufferPoolFull,
    InvalidPageId(u64),
    AccessDenied(String),
    PathTraversalAttempt(String),
    IoError(String),
}

/// Zero-Copy Buffer (04-060)
/// Wraps contiguous byte storage and provides zero-copy sub-slices sharing the backing allocation.
#[derive(Clone)]
pub struct ZeroCopyBuffer {
    data: Bytes,
}

impl ZeroCopyBuffer {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Bytes::from(data),
        }
    }

    pub fn slice(&self, start: usize, end: usize) -> ZeroCopyBuffer {
        Self {
            data: self.data.slice(start..end),
        }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

/// Buffer Manager / Fixed-size Page Pool with LRU Eviction & Pinning (04-061)
#[derive(Debug)]
pub struct PageFrame {
    pub page_id: u64,
    pub data: Vec<u8>,
    pub pin_count: AtomicUsize,
    pub is_dirty: AtomicBool,
}

pub struct BufferManager {
    capacity_pages: usize,
    page_size: usize,
    frames: std::sync::RwLock<HashMap<u64, Arc<PageFrame>>>,
    lru_order: std::sync::Mutex<VecDeque<u64>>,
    pub hits: AtomicU64,
    pub misses: AtomicU64,
}

impl BufferManager {
    pub fn new(capacity_pages: usize, page_size: usize) -> Self {
        Self {
            capacity_pages,
            page_size,
            frames: std::sync::RwLock::new(HashMap::new()),
            lru_order: std::sync::Mutex::new(VecDeque::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    pub fn get_or_fetch_page(&self, page_id: u64) -> Result<Arc<PageFrame>, StorageError> {
        {
            let frames = self.frames.read().unwrap();
            if let Some(frame) = frames.get(&page_id) {
                frame.pin_count.fetch_add(1, Ordering::SeqCst);
                self.hits.fetch_add(1, Ordering::Relaxed);
                let mut lru = self.lru_order.lock().unwrap();
                lru.retain(|&id| id != page_id);
                lru.push_back(page_id);
                return Ok(Arc::clone(frame));
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);

        // Evict if at capacity
        let mut frames = self.frames.write().unwrap();
        let mut lru = self.lru_order.lock().unwrap();

        if frames.len() >= self.capacity_pages {
            // Find unpinned candidate
            let mut evicted_id = None;
            for (idx, &candidate_id) in lru.iter().enumerate() {
                if let Some(frame) = frames.get(&candidate_id) {
                    if frame.pin_count.load(Ordering::SeqCst) == 0 {
                        evicted_id = Some((idx, candidate_id));
                        break;
                    }
                }
            }

            if let Some((idx, evict_id)) = evicted_id {
                lru.remove(idx);
                frames.remove(&evict_id);
            } else {
                return Err(StorageError::BufferPoolFull);
            }
        }

        let new_frame = Arc::new(PageFrame {
            page_id,
            data: vec![0u8; self.page_size],
            pin_count: AtomicUsize::new(1),
            is_dirty: AtomicBool::new(false),
        });

        frames.insert(page_id, Arc::clone(&new_frame));
        lru.push_back(page_id);
        Ok(new_frame)
    }

    pub fn unpin_page(&self, page_id: u64) {
        let frames = self.frames.read().unwrap();
        if let Some(frame) = frames.get(&page_id) {
            let current = frame.pin_count.load(Ordering::SeqCst);
            if current > 0 {
                frame.pin_count.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed) as f64;
        let m = self.misses.load(Ordering::Relaxed) as f64;
        if h + m == 0.0 {
            0.0
        } else {
            h / (h + m)
        }
    }
}

/// Budgeted Memory Pool (04-062)
pub struct MemoryPool {
    max_bytes: usize,
    allocated_bytes: AtomicUsize,
}

impl MemoryPool {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            allocated_bytes: AtomicUsize::new(0),
        }
    }

    pub fn reserve(&self, bytes: usize) -> Result<(), StorageError> {
        let mut cur = self.allocated_bytes.load(Ordering::SeqCst);
        loop {
            if cur + bytes > self.max_bytes {
                return Err(StorageError::OutOfMemory {
                    requested: bytes,
                    available: self.max_bytes.saturating_sub(cur),
                });
            }
            match self.allocated_bytes.compare_exchange_weak(
                cur,
                cur + bytes,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => cur = actual,
            }
        }
    }

    pub fn release(&self, bytes: usize) {
        let _ = self
            .allocated_bytes
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |cur| {
                Some(cur.saturating_sub(bytes))
            });
    }

    pub fn allocated(&self) -> usize {
        self.allocated_bytes.load(Ordering::SeqCst)
    }
}

/// Local Filesystem Connector with Path Traversal Protection (04-076)
pub struct LocalFsConnector {
    root_dir: PathBuf,
}

impl LocalFsConnector {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    pub fn list_files_matching_glob(
        &self,
        glob_pattern: &str,
    ) -> Result<Vec<PathBuf>, StorageError> {
        if glob_pattern.contains("../") || glob_pattern.contains("..\\") {
            return Err(StorageError::PathTraversalAttempt(glob_pattern.to_string()));
        }

        let mut matches = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.root_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if glob_pattern == "*" || file_name.ends_with(glob_pattern.trim_start_matches('*'))
                {
                    matches.push(path);
                }
            }
        }
        Ok(matches)
    }

    pub fn infer_csv_schema(&self, path: &Path) -> Result<Vec<(String, String)>, StorageError> {
        // Traversal guard
        if !path.starts_with(&self.root_dir) {
            return Err(StorageError::PathTraversalAttempt(
                path.display().to_string(),
            ));
        }

        let content =
            std::fs::read_to_string(path).map_err(|e| StorageError::IoError(e.to_string()))?;
        let header = content.lines().next().unwrap_or("");
        let cols = header
            .split(',')
            .map(|c| (c.trim().to_string(), "Utf8".to_string()))
            .collect();
        Ok(cols)
    }
}
