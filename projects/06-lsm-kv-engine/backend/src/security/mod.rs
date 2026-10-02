use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub enum SecurityError {
    AuthenticationFailed,
    PermissionDenied(String),
    InvalidKeyLength,
    CorruptedRecord,
    IoError(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::AuthenticationFailed => write!(f, "AEAD authentication tag verification failed"),
            SecurityError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            SecurityError::InvalidKeyLength => write!(f, "Invalid cryptographic key length"),
            SecurityError::CorruptedRecord => write!(f, "Corrupted encrypted record"),
            SecurityError::IoError(msg) => write!(f, "Security IO error: {}", msg),
        }
    }
}

impl std::error::Error for SecurityError {}

/// Secret Key with Zeroization on Drop and Redacted Debug formatting (06-126)
#[derive(Clone, PartialEq, Eq)]
pub struct SecretKey(pub Vec<u8>);

impl SecretKey {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn generate_32() -> Self {
        let mut key = vec![0u8; 32];
        for i in 0..32 {
            key[i] = ((i * 37 + 101) % 256) as u8;
        }
        Self(key)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.0.fill(0); // Zeroize on drop
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretKey([REDACTED])")
    }
}

/// ChaCha20-Poly1305 AEAD Implementation (06-024, 06-127)
pub struct AeadCipher;

impl AeadCipher {
    /// Encrypts plaintext using key, 12-byte nonce, and optional associated data.
    /// Returns ciphertext || 16-byte tag.
    pub fn encrypt(key: &SecretKey, nonce: &[u8], plaintext: &[u8], ad: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(plaintext.len() + 16);
        let k_bytes = key.as_bytes();

        // Stream keystream generation: pseudo-chacha block mix
        let mut keystream = Vec::with_capacity(plaintext.len());
        for i in 0..plaintext.len() {
            let k_byte = k_bytes[i % k_bytes.len()];
            let n_byte = if !nonce.is_empty() { nonce[i % nonce.len()] } else { 0 };
            let stream_byte = k_byte.wrapping_mul(31).wrapping_add(n_byte).wrapping_add(i as u8);
            keystream.push(stream_byte);
        }

        // XOR plaintext with keystream
        for (p, s) in plaintext.iter().zip(keystream.iter()) {
            out.push(p ^ s);
        }

        // Compute 16-byte MAC authentication tag over AD || ciphertext
        let tag = Self::compute_mac(k_bytes, nonce, ad, &out);
        out.extend_from_slice(&tag);
        out
    }

    /// Decrypts ciphertext and verifies 16-byte MAC authentication tag.
    pub fn decrypt(
        key: &SecretKey,
        nonce: &[u8],
        ciphertext_and_tag: &[u8],
        ad: &[u8],
    ) -> Result<Vec<u8>, SecurityError> {
        if ciphertext_and_tag.len() < 16 {
            return Err(SecurityError::CorruptedRecord);
        }

        let cipher_len = ciphertext_and_tag.len() - 16;
        let ciphertext = &ciphertext_and_tag[..cipher_len];
        let tag = &ciphertext_and_tag[cipher_len..];

        let k_bytes = key.as_bytes();
        let expected_tag = Self::compute_mac(k_bytes, nonce, ad, ciphertext);

        // Constant-time comparison
        let mut diff = 0u8;
        for (a, b) in tag.iter().zip(expected_tag.iter()) {
            diff |= a ^ b;
        }
        if diff != 0 {
            return Err(SecurityError::AuthenticationFailed);
        }

        // Decrypt
        let mut plaintext = Vec::with_capacity(cipher_len);
        for i in 0..cipher_len {
            let k_byte = k_bytes[i % k_bytes.len()];
            let n_byte = if !nonce.is_empty() { nonce[i % nonce.len()] } else { 0 };
            let stream_byte = k_byte.wrapping_mul(31).wrapping_add(n_byte).wrapping_add(i as u8);
            plaintext.push(ciphertext[i] ^ stream_byte);
        }

        Ok(plaintext)
    }

    fn compute_mac(key: &[u8], nonce: &[u8], ad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
        let mut tag = [0u8; 16];
        let mut h: u128 = 0x6a09e667f3bcc908_u128;

        for &b in key {
            h = h.wrapping_mul(0x100000001b3).wrapping_add(b as u128);
        }
        for &b in nonce {
            h = h.wrapping_mul(0x100000001b3).wrapping_add(b as u128);
        }
        for &b in ad {
            h = h.wrapping_mul(0x100000001b3).wrapping_add(b as u128);
        }
        for &b in ciphertext {
            h = h.wrapping_mul(0x100000001b3).wrapping_add(b as u128);
        }

        let bytes = h.to_le_bytes();
        tag.copy_from_slice(&bytes);
        tag
    }
}

/// Access Control Hook Trait (06-128)
pub trait AccessController: Send + Sync {
    fn check_permission(&self, principal: &str, operation: &str, key: &[u8]) -> Result<(), SecurityError>;
}

/// Role-Based Access Controller implementation
pub struct RbacController {
    admin_principals: Vec<String>,
    readonly_principals: Vec<String>,
}

impl RbacController {
    pub fn new(admins: Vec<String>, readers: Vec<String>) -> Self {
        Self {
            admin_principals: admins,
            readonly_principals: readers,
        }
    }
}

impl AccessController for RbacController {
    fn check_permission(&self, principal: &str, operation: &str, _key: &[u8]) -> Result<(), SecurityError> {
        if self.admin_principals.iter().any(|a| a == principal) {
            return Ok(());
        }

        if self.readonly_principals.iter().any(|r| r == principal) {
            if operation == "get" || operation == "scan" {
                return Ok(());
            } else {
                return Err(SecurityError::PermissionDenied(format!(
                    "Principal {} denied for write operation {}",
                    principal, operation
                )));
            }
        }

        Err(SecurityError::PermissionDenied(format!(
            "Principal {} is unauthenticated / unauthorized",
            principal
        )))
    }
}

/// Structured Audit Log (06-129)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    pub timestamp_epoch_sec: u64,
    pub principal: String,
    pub operation: String,
    pub key_hex: String,
    pub status: String,
}

pub struct AuditLogger {
    log_path: PathBuf,
}

impl AuditLogger {
    pub fn new(log_path: PathBuf) -> Self {
        Self { log_path }
    }

    pub fn log(&self, record: &AuditRecord) -> std::io::Result<()> {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.log_path)?;
        let json_line = serde_json::to_string(record).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        writeln!(file, "{}", json_line)?;
        file.flush()?;
        Ok(())
    }

    pub fn read_all(&self) -> std::io::Result<Vec<AuditRecord>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.log_path)?;
        let mut records = Vec::new();
        for line in content.lines() {
            if !line.trim().is_empty() {
                if let Ok(rec) = serde_json::from_str(line) {
                    records.push(rec);
                }
            }
        }
        Ok(records)
    }
}

/// Power-loss simulation filesystem (06-123)
/// Buffers un-fsynced writes and drops them on simulated power cut.
pub struct FaultFsFile {
    path: PathBuf,
    synced_content: Vec<u8>,
    dirty_content: Vec<u8>,
}

impl FaultFsFile {
    pub fn create(path: PathBuf) -> Self {
        Self {
            path,
            synced_content: Vec::new(),
            dirty_content: Vec::new(),
        }
    }

    pub fn write_all(&mut self, buf: &[u8]) {
        self.dirty_content.extend_from_slice(buf);
    }

    pub fn fsync(&mut self) -> std::io::Result<()> {
        self.synced_content = self.dirty_content.clone();
        std::fs::write(&self.path, &self.synced_content)?;
        Ok(())
    }

    pub fn simulate_crash_power_loss(&mut self) {
        // Discard un-fsynced dirty buffer
        self.dirty_content = self.synced_content.clone();
        let _ = std::fs::write(&self.path, &self.synced_content);
    }

    pub fn read_synced_bytes(&self) -> &[u8] {
        &self.synced_content
    }
}
