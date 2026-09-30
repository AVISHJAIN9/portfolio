use std::collections::HashMap;

/// Authoritative checksum comparator and divergence detector.
/// Complies with FR-010 and FR-011.
#[derive(Debug, Default)]
pub struct DesyncDetector {
    pub authoritative_hashes: HashMap<u64, u32>,
}

impl DesyncDetector {
    pub fn new() -> Self {
        Self {
            authoritative_hashes: HashMap::new(),
        }
    }

    pub fn record_authoritative_hash(&mut self, frame: u64, hash: u32) {
        self.authoritative_hashes.insert(frame, hash);
    }

    pub fn verify_client_hash(&self, frame: u64, client_hash: u32) -> Result<bool, &'static str> {
        if let Some(&auth_hash) = self.authoritative_hashes.get(&frame) {
            Ok(auth_hash == client_hash)
        } else {
            Err("Frame checksum not available in server history")
        }
    }
}
