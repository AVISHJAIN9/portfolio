/// Cryptographic HMAC / Keyed Hash using Blake3 in Keyed Mode (RFC standard).
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
    fn test_keyed_mac() {
        let key = [0x42u8; 32];
        let msg = b"authenticated_session_token_data";
        let mac = keyed_mac(&key, msg);
        assert!(verify_mac(&key, msg, &mac));
        assert!(!verify_mac(&key, b"tampered_token_data", &mac));
    }
}
