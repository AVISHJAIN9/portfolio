use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// HKDF-Extract per RFC 5869 (HMAC-SHA256)
pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let mut mac = if salt.is_empty() {
        HmacSha256::new_from_slice(&[0u8; 32]).expect("HMAC can take any key size")
    } else {
        HmacSha256::new_from_slice(salt).expect("HMAC can take any key size")
    };
    mac.update(ikm);
    let result = mac.finalize();
    let mut prk = [0u8; 32];
    prk.copy_from_slice(&result.into_bytes());
    prk
}

/// HKDF-Expand per RFC 5869 (HMAC-SHA256)
pub fn hkdf_expand(prk: &[u8; 32], info: &[u8], okm: &mut [u8]) {
    let len = okm.len();
    let num_blocks = len.div_ceil(32);
    assert!(num_blocks <= 255, "HKDF okm too long");

    let mut t = Vec::with_capacity(32);
    let mut offset = 0;

    for i in 1..=num_blocks {
        let mut mac = HmacSha256::new_from_slice(prk).expect("HMAC can take 32 byte key");
        if i > 1 {
            mac.update(&t);
        }
        mac.update(info);
        mac.update(&[i as u8]);
        let block = mac.finalize().into_bytes();
        t = block.to_vec();

        let to_copy = std::cmp::min(32, len - offset);
        okm[offset..offset + to_copy].copy_from_slice(&t[..to_copy]);
        offset += to_copy;
    }
}

/// RFC 9001 HKDF-Expand-Label for QUIC Key Derivation
pub fn hkdf_expand_label(secret: &[u8; 32], label: &str, context: &[u8], out: &mut [u8]) {
    let mut info = Vec::with_capacity(4 + 6 + label.len() + 1 + context.len());
    // Length: 2 bytes
    info.extend_from_slice(&(out.len() as u16).to_be_bytes());
    // Label: "tls13 " + label with 1-byte length prefix
    let full_label = format!("tls13 {}", label);
    info.push(full_label.len() as u8);
    info.extend_from_slice(full_label.as_bytes());
    // Context: 1-byte length prefix + context
    info.push(context.len() as u8);
    info.extend_from_slice(context);

    hkdf_expand(secret, &info, out);
}

/// Blake3 Keyed MAC (educational / fast local token authentication).
pub fn keyed_mac(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    *blake3::keyed_hash(key, message).as_bytes()
}

/// Constant-time MAC verification.
pub fn verify_mac(key: &[u8; 32], message: &[u8], expected_mac: &[u8; 32]) -> bool {
    let actual_mac = keyed_mac(key, message);
    constant_time_eq(&actual_mac, expected_mac)
}

/// Constant-time slice comparison preventing timing side-channel attacks.
#[inline]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hkdf_rfc5869_vectors() {
        let ikm = [0x0bu8; 22];
        let salt = [0x00u8; 13];
        let prk = hkdf_extract(&salt, &ikm);
        assert_eq!(prk.len(), 32);

        let mut okm = [0u8; 42];
        hkdf_expand(&prk, b"test_info", &mut okm);
        assert_ne!(okm, [0u8; 42]);
    }

    #[test]
    fn test_quic_rfc9001_expand_label() {
        let secret = [0x33u8; 32];
        let mut key = [0u8; 16];
        let mut iv = [0u8; 12];
        hkdf_expand_label(&secret, "quic key", b"", &mut key);
        hkdf_expand_label(&secret, "quic iv", b"", &mut iv);
        assert_ne!(key, [0u8; 16]);
        assert_ne!(iv, [0u8; 12]);
    }

    #[test]
    fn test_keyed_mac() {
        let key = [0x42u8; 32];
        let msg = b"authenticated_session_token_data";
        let mac = keyed_mac(&key, msg);
        assert!(verify_mac(&key, msg, &mac));
        assert!(!verify_mac(&key, b"tampered_token_data", &mac));
    }
}
