use crate::engine::LsmEngine;
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::Ordering;

#[derive(Debug, PartialEq, Eq)]
pub enum TxnError {
    Conflict,
    AlreadyCommitted,
    Aborted,
    IoError(String),
}

/// Transaction with Snapshot Isolation and Optimistic Concurrency Control (06-010, 06-088, 06-089)
pub struct Transaction<'a> {
    engine: &'a LsmEngine,
    pub start_seq: u64,
    pub read_set: HashSet<Vec<u8>>,
    pub write_set: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    committed: bool,
}

impl<'a> Transaction<'a> {
    pub fn new(engine: &'a LsmEngine) -> Self {
        let start_seq = engine.snapshot();
        Self {
            engine,
            start_seq,
            read_set: HashSet::new(),
            write_set: BTreeMap::new(),
            committed: false,
        }
    }

    pub fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, TxnError> {
        if self.committed {
            return Err(TxnError::AlreadyCommitted);
        }
        self.read_set.insert(key.to_vec());

        if let Some(val_opt) = self.write_set.get(key) {
            return Ok(val_opt.clone());
        }

        self.engine.get(key).map_err(|e| TxnError::IoError(e.to_string()))
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), TxnError> {
        if self.committed {
            return Err(TxnError::AlreadyCommitted);
        }
        self.write_set.insert(key.to_vec(), Some(value.to_vec()));
        Ok(())
    }

    pub fn delete(&mut self, key: &[u8]) -> Result<(), TxnError> {
        if self.committed {
            return Err(TxnError::AlreadyCommitted);
        }
        self.write_set.insert(key.to_vec(), None);
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), TxnError> {
        if self.committed {
            return Err(TxnError::AlreadyCommitted);
        }
        self.write_set.clear();
        self.read_set.clear();
        self.committed = true;
        Ok(())
    }

    /// Commit transaction with conflict detection
    pub fn commit(&mut self) -> Result<(), TxnError> {
        if self.committed {
            return Err(TxnError::AlreadyCommitted);
        }

        // Conflict check: if any key in read_set or write_set was modified after start_seq
        let current_seq = self.engine.snapshot();
        if current_seq > self.start_seq {
            // Check conflicts against modified keys
            for k in self.read_set.iter().chain(self.write_set.keys()) {
                if self.engine.is_key_modified_since(k, self.start_seq) {
                    return Err(TxnError::Conflict);
                }
            }
        }

        // Apply atomic batch
        let puts: Vec<(&[u8], &[u8])> = self
            .write_set
            .iter()
            .filter_map(|(k, v)| v.as_ref().map(|val| (k.as_slice(), val.as_slice())))
            .collect();
        let deletes: Vec<&[u8]> = self
            .write_set
            .iter()
            .filter_map(|(k, v)| if v.is_none() { Some(k.as_slice()) } else { None })
            .collect();

        self.engine
            .write_batch(&puts, &deletes)
            .map_err(|e| TxnError::IoError(e.to_string()))?;

        self.committed = true;
        Ok(())
    }
}
