use hmac::{Hmac, Mac};
use sha2::Sha256;

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

    /// RFC 5869 Appendix A.1 Test Case 1
    #[test]
    fn test_rfc5869_test_case_1() {
        let ikm = [0x0bu8; 22];
        let salt = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];
        let info = [0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9];
        
        let expected_prk = [
            0x07, 0x77, 0x09, 0x36, 0x2c, 0x2e, 0x32, 0xdf, 0x0d, 0xdc, 0x3f, 0x0d, 0xc4, 0x7b,
            0xba, 0x63, 0x90, 0xb6, 0xc7, 0x3b, 0xb5, 0x0f, 0x9c, 0x31, 0x22, 0xec, 0x84, 0x4a,
            0xd7, 0xc2, 0xb3, 0xe5,
        ];
        let prk = hkdf_extract(&salt, &ikm);
        assert_eq!(prk, expected_prk);

        let mut okm = [0u8; 42];
        hkdf_expand(&prk, &info, &mut okm);
        let expected_okm_prefix = [
            0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36,
            0x2f, 0x2a,
        ];
        assert_eq!(&okm[..16], &expected_okm_prefix);
    }

    /// RFC 9001 Appendix A.1 Test Vectors (DCID: 0x8394c8f03e515708)
    #[test]
    fn test_rfc9001_initial_secrets() {
        let dcid = [0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08];
        let initial_salt = [
            0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8,
            0x0c, 0xad, 0xcc, 0xbb, 0x7f, 0x0a,
        ];
        let initial_secret = hkdf_extract(&initial_salt, &dcid);
        let expected_initial_secret = [
            0x7d, 0xb5, 0xdf, 0x06, 0xe7, 0xa6, 0x9e, 0x43, 0x24, 0x96, 0xad, 0xed, 0xb0, 0x08,
            0x51, 0x92, 0x35, 0x95, 0x22, 0x15, 0x96, 0xae, 0x2a, 0xe9, 0xfb, 0x81, 0x15, 0xc1,
            0xe9, 0xed, 0x0a, 0x44,
        ];
        assert_eq!(initial_secret, expected_initial_secret);

        let mut client_initial_secret = [0u8; 32];
        hkdf_expand_label(&initial_secret, "client in", b"", &mut client_initial_secret);
        let expected_client_initial_secret = [
            0xc0, 0x0c, 0xf1, 0x51, 0xca, 0x5b, 0xe0, 0x75, 0xed, 0x0e, 0xbf, 0xb5, 0xc8, 0x03,
            0x23, 0xc4, 0x2d, 0x6b, 0x7d, 0xb6, 0x78, 0x81, 0x28, 0x9a, 0xf4, 0x00, 0x8f, 0x1f,
            0x6c, 0x35, 0x7a, 0xea,
        ];
        assert_eq!(client_initial_secret, expected_client_initial_secret);
    }
}
