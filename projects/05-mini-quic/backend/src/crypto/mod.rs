pub mod handshake;
pub mod tls;

use pf_crypto::{hkdf_expand_label, hkdf_extract, keyed_mac, verify_mac};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// RFC 9001 QUIC v1 Initial Salt
pub const QUIC_V1_INITIAL_SALT: [u8; 20] = [
    0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c, 0xad,
    0xcc, 0xbb, 0x7f, 0x0a,
];

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
    pub client_initial_secret: [u8; 32],
    pub server_initial_secret: [u8; 32],
    pub application_secret: Option<[u8; 32]>,
}

impl CryptoContext {
    /// Derives Initial Secrets from Destination Connection ID per RFC 9001 Section 5.2
    pub fn new_initial(dcid: &[u8]) -> Self {
        let initial_secret = hkdf_extract(&QUIC_V1_INITIAL_SALT, dcid);

        let mut client_initial_secret = [0u8; 32];
        let mut server_initial_secret = [0u8; 32];

        hkdf_expand_label(
            &initial_secret,
            "client in",
            b"",
            &mut client_initial_secret,
        );
        hkdf_expand_label(
            &initial_secret,
            "server in",
            b"",
            &mut server_initial_secret,
        );

        Self {
            initial_secret,
            client_initial_secret,
            server_initial_secret,
            application_secret: None,
        }
    }

    pub fn derive_application_keys(&mut self, master_secret: &[u8; 32]) {
        let mut app_secret = [0u8; 32];
        hkdf_expand_label(master_secret, "quic app", b"", &mut app_secret);
        self.application_secret = Some(app_secret);
    }

    /// Educational / Diagnostic packet MAC authentication (Blake3 keyed mode)
    pub fn compute_tag(&self, message: &[u8]) -> [u8; 32] {
        let key = self
            .application_secret
            .as_ref()
            .unwrap_or(&self.initial_secret);
        keyed_mac(key, message)
    }

    pub fn verify_tag(&self, message: &[u8], expected: &[u8; 32]) -> bool {
        let key = self
            .application_secret
            .as_ref()
            .unwrap_or(&self.initial_secret);
        verify_mac(key, message, expected)
    }
}
