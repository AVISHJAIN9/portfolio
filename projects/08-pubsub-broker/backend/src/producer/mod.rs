//! Idempotent Producer & Sequence Tracking (Features 08-066..08-078)

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProducerError {
    DuplicateSequence { producer_id: u64, sequence: u64 },
    SequenceOutOfOrder { producer_id: u64, expected: u64, got: u64 },
    FencedEpoch { producer_id: u64, current_epoch: u32, got_epoch: u32 },
}

#[derive(Debug, Clone)]
struct ProducerSession {
    epoch: u32,
    last_sequence: u64,
}

/// 08-066..08-078: Idempotent Producer Tracker
pub struct IdempotentProducerRegistry {
    producers: RwLock<HashMap<u64, ProducerSession>>,
}

impl Default for IdempotentProducerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl IdempotentProducerRegistry {
    pub fn new() -> Self {
        Self {
            producers: RwLock::new(HashMap::new()),
        }
    }

    /// 08-067: Init producer ID and epoch
    pub fn init_producer(&self, producer_id: u64, epoch: u32) {
        let mut map = self.producers.write();
        map.insert(producer_id, ProducerSession { epoch, last_sequence: 0 });
    }

    /// 08-068..08-071: Verify sequence for exactly-once/idempotent publishing
    pub fn verify_and_advance_sequence(&self, producer_id: u64, epoch: u32, sequence: u64) -> Result<(), ProducerError> {
        let mut map = self.producers.write();
        let session = map.entry(producer_id).or_insert(ProducerSession {
            epoch,
            last_sequence: 0,
        });

        // 08-070: Epoch fencing
        if epoch < session.epoch {
            return Err(ProducerError::FencedEpoch {
                producer_id,
                current_epoch: session.epoch,
                got_epoch: epoch,
            });
        }
        if epoch > session.epoch {
            session.epoch = epoch;
            session.last_sequence = 0;
        }

        // 08-068: Duplicate check
        if sequence > 0 && sequence <= session.last_sequence {
            return Err(ProducerError::DuplicateSequence { producer_id, sequence });
        }

        // 08-069: Out-of-order check (strict monotonic sequence starting from 1)
        if sequence != session.last_sequence + 1 && !(session.last_sequence == 0 && sequence == 1) {
            return Err(ProducerError::SequenceOutOfOrder {
                producer_id,
                expected: session.last_sequence + 1,
                got: sequence,
            });
        }

        session.last_sequence = sequence;
        Ok(())
    }

    pub fn last_sequence(&self, producer_id: u64) -> Option<u64> {
        self.producers.read().get(&producer_id).map(|s| s.last_sequence)
    }
}
