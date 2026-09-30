//! QUIC Cryptographic Key Management and 0-RTT Session Resumption

use pf_crypto::{keyed_mac, verify_mac};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct SessionTicket {
    pub ticket_id: Vec<u8>,
    pub secret: [u8; 32],
    pub max_early_data: u32,
    pub created_at: Instant,
    pub lifetime: Duration,
}

pub struct SessionCache {
    tickets: Mutex<HashMap<Vec<u8>, SessionTicket>>,
}

impl Default for SessionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionCache {
    pub fn new() -> Self {
        Self {
            tickets: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, ticket: SessionTicket) {
        let mut guard = self.tickets.lock().unwrap();
        guard.insert(ticket.ticket_id.clone(), ticket);
    }

    pub fn get(&self, ticket_id: &[u8]) -> Option<SessionTicket> {
        let mut guard = self.tickets.lock().unwrap();
        if let Some(ticket) = guard.get(ticket_id) {
            if ticket.created_at.elapsed() <= ticket.lifetime {
                return Some(ticket.clone());
            } else {
                guard.remove(ticket_id);
            }
        }
        None
    }
}

pub struct CryptoContext {
    pub initial_secret: [u8; 32],
    pub handshake_secret: Option<[u8; 32]>,
    pub application_secret: Option<[u8; 32]>,
}

impl CryptoContext {
    pub fn new_initial(dcid: &[u8]) -> Self {
        let salt = b"mini_quic_v1_initial_salt";
        let mut seed = Vec::with_capacity(salt.len() + dcid.len());
        seed.extend_from_slice(salt);
        seed.extend_from_slice(dcid);
        
        let hash = blake3::hash(&seed);
        let mut initial_secret = [0u8; 32];
        initial_secret.copy_from_slice(hash.as_bytes());

        Self {
            initial_secret,
            handshake_secret: None,
            application_secret: None,
        }
    }

    pub fn derive_application_keys(&mut self, master_secret: &[u8; 32]) {
        let app_hash = keyed_mac(master_secret, b"quic_app_traffic_secret");
        self.application_secret = Some(app_hash);
    }

    pub fn compute_tag(&self, message: &[u8]) -> [u8; 32] {
        let key = self.application_secret.as_ref().unwrap_or(&self.initial_secret);
        keyed_mac(key, message)
    }

    pub fn verify_tag(&self, message: &[u8], expected: &[u8; 32]) -> bool {
        let key = self.application_secret.as_ref().unwrap_or(&self.initial_secret);
        verify_mac(key, message, expected)
    }
}
