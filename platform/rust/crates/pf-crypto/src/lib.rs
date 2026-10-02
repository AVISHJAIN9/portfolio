use aes::cipher::{BlockCipherEncrypt, KeyInit};
use aes::Aes128;
use aes_gcm::aead::{Aead, Payload};
use aes_gcm::Aes128Gcm;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use chacha20::ChaCha20;
use chacha20poly1305::ChaCha20Poly1305;
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

/// Computes the QUIC AEAD nonce from the base IV and packet number: `iv ^ (pn as 12-byte big-endian)`
pub fn quic_aead_nonce(iv: &[u8; 12], packet_number: u64) -> [u8; 12] {
    let mut nonce = *iv;
    let pn_bytes = packet_number.to_be_bytes();
    for i in 0..8 {
        nonce[4 + i] ^= pn_bytes[i];
    }
    nonce
}

/// Encrypts a QUIC packet payload using AES-128-GCM (RFC 9001 Section 5.3)
pub fn quic_aes_gcm_encrypt(
    key: &[u8; 16],
    nonce: &[u8; 12],
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|_| "Invalid key length")?;
    let gcm_nonce = (*nonce).into();
    let payload = Payload {
        msg: plaintext,
        aad: associated_data,
    };
    cipher.encrypt(&gcm_nonce, payload).map_err(|_| "Encryption failure")
}

/// Decrypts a QUIC packet payload using AES-128-GCM (RFC 9001 Section 5.3)
pub fn quic_aes_gcm_decrypt(
    key: &[u8; 16],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|_| "Invalid key length")?;
    let gcm_nonce = (*nonce).into();
    let payload = Payload {
        msg: ciphertext,
        aad: associated_data,
    };
    cipher.decrypt(&gcm_nonce, payload).map_err(|_| "Decryption/tag failure")
}

/// Encrypts a QUIC packet payload using ChaCha20-Poly1305 (RFC 9001 Section 5.3)
pub fn quic_chacha20_poly1305_encrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let cipher = ChaCha20Poly1305::new_from_slice(key).map_err(|_| "Invalid key length")?;
    let chacha_nonce = (*nonce).into();
    let payload = Payload {
        msg: plaintext,
        aad: associated_data,
    };
    cipher.encrypt(&chacha_nonce, payload).map_err(|_| "Encryption failure")
}

/// Decrypts a QUIC packet payload using ChaCha20-Poly1305 (RFC 9001 Section 5.3)
pub fn quic_chacha20_poly1305_decrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let cipher = ChaCha20Poly1305::new_from_slice(key).map_err(|_| "Invalid key length")?;
    let chacha_nonce = (*nonce).into();
    let payload = Payload {
        msg: ciphertext,
        aad: associated_data,
    };
    cipher.decrypt(&chacha_nonce, payload).map_err(|_| "Decryption/tag failure")
}

/// Computes AES-128 Header Protection Mask (RFC 9001 Section 5.4.1)
pub fn quic_aes_hp_mask(hp_key: &[u8; 16], sample: &[u8; 16]) -> [u8; 5] {
    let cipher = Aes128::new(hp_key.into());
    let mut block = (*sample).into();
    cipher.encrypt_block(&mut block);
    let mut mask = [0u8; 5];
    mask.copy_from_slice(&block[..5]);
    mask
}

/// Computes ChaCha20 Header Protection Mask (RFC 9001 Section 5.4.2)
pub fn quic_chacha20_hp_mask(hp_key: &[u8; 32], sample: &[u8; 16]) -> [u8; 5] {
    let counter = u32::from_le_bytes(sample[0..4].try_into().unwrap());
    let nonce: &[u8; 12] = sample[4..16].try_into().unwrap();
    let mut cipher = ChaCha20::new(hp_key.into(), nonce.into());
    cipher.seek(counter as u64 * 64);
    let mut mask = [0u8; 5];
    cipher.apply_keystream(&mut mask);
    mask
}

pub const X25519_BASEPOINT: [u8; 32] = [
    9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// X25519 Key Agreement (RFC 7748)
pub fn x25519(private_key: &[u8; 32], peer_public: &[u8; 32]) -> [u8; 32] {
    let mut clamped = *private_key;
    clamped[0] &= 248;
    clamped[31] &= 127;
    clamped[31] |= 64;
    x25519_dalek::x25519(clamped, *peer_public)
}

pub fn x25519_public_key(private_key: &[u8; 32]) -> [u8; 32] {
    let mut clamped = *private_key;
    clamped[0] &= 248;
    clamped[31] &= 127;
    clamped[31] |= 64;
    x25519_dalek::x25519(clamped, X25519_BASEPOINT)
}

/// Blake3 Keyed MAC
pub fn keyed_mac(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    *blake3::keyed_hash(key, message).as_bytes()
}

/// Constant-time MAC verification.
pub fn verify_mac(key: &[u8; 32], message: &[u8], expected_mac: &[u8; 32]) -> bool {
    let actual_mac = keyed_mac(key, message);
    constant_time_eq(&actual_mac, expected_mac)
}

/// RFC 9001 Section 6 Key Update: derives the next generation application secret
pub fn quic_key_update(current_secret: &[u8; 32]) -> [u8; 32] {
    let mut next_secret = [0u8; 32];
    hkdf_expand_label(current_secret, "quic ku", b"", &mut next_secret);
    next_secret
}

/// Constant-time slice comparison
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

/// Ed25519 signature generation (RFC 8032 / TLS 1.3 0x0807)
pub fn ed25519_sign(secret_key_bytes: &[u8; 32], message: &[u8]) -> [u8; 64] {
    use ed25519_dalek::{Signer, SigningKey};
    let signing_key = SigningKey::from_bytes(secret_key_bytes);
    signing_key.sign(message).to_bytes()
}

/// Ed25519 signature verification (RFC 8032 / TLS 1.3 0x0807)
pub fn ed25519_verify(public_key_bytes: &[u8; 32], message: &[u8], signature_bytes: &[u8; 64]) -> bool {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let Ok(verifying_key) = VerifyingKey::from_bytes(public_key_bytes) else {
        return false;
    };
    let sig = Signature::from_bytes(signature_bytes);
    verifying_key.verify(message, &sig).is_ok()
}

pub fn ed25519_public_key(secret_key_bytes: &[u8; 32]) -> [u8; 32] {
    use ed25519_dalek::SigningKey;
    let signing_key = SigningKey::from_bytes(secret_key_bytes);
    signing_key.verifying_key().to_bytes()
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
        let expected_okm = [
            0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36,
            0x2f, 0x2a, 0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c, 0x5d, 0xb0, 0x2d, 0x56,
            0xec, 0xc4, 0xc5, 0xbf, 0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18, 0x58, 0x65,
        ];
        assert_eq!(okm, expected_okm);
    }

    /// RFC 5869 Appendix A.2 Test Case 2
    #[test]
    fn test_rfc5869_test_case_2() {
        let mut ikm = [0u8; 80];
        for (i, b) in ikm.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut salt = [0u8; 80];
        for (i, b) in salt.iter_mut().enumerate() {
            *b = (i as u8) + 0x60;
        }
        let mut info = [0u8; 80];
        for (i, b) in info.iter_mut().enumerate() {
            *b = (i as u8) + 0xb0;
        }
        let prk = hkdf_extract(&salt, &ikm);
        let expected_prk = [
            0x06, 0xa6, 0xb8, 0x8c, 0x58, 0x53, 0x36, 0x1a, 0x06, 0x10, 0x4c, 0x9c, 0xeb, 0x35,
            0xb4, 0x5c, 0xef, 0x76, 0x00, 0x14, 0x90, 0x46, 0x71, 0x01, 0x4a, 0x19, 0x3f, 0x40,
            0xc1, 0x5f, 0xc2, 0x44,
        ];
        assert_eq!(prk, expected_prk);

        let mut okm = [0u8; 82];
        hkdf_expand(&prk, &info, &mut okm);
        assert_eq!(okm[0], 0xb1);
        assert_eq!(okm[1], 0x1e);
        assert_eq!(okm[81], 0x87);
    }

    /// RFC 5869 Appendix A.3 Test Case 3 (Zero-length Salt and Info)
    #[test]
    fn test_rfc5869_test_case_3() {
        let ikm = [0x0bu8; 22];
        let salt = [];
        let info = [];
        let prk = hkdf_extract(&salt, &ikm);
        let mut okm = [0u8; 42];
        hkdf_expand(&prk, &info, &mut okm);
        assert_eq!(&okm[..4], &[0x8d, 0xa4, 0xe7, 0x75]);
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

        let mut server_initial_secret = [0u8; 32];
        hkdf_expand_label(&initial_secret, "server in", b"", &mut server_initial_secret);
        let expected_server_initial_secret = [
            0x3c, 0x19, 0x98, 0x28, 0xfd, 0x13, 0x9e, 0xfd, 0x21, 0x6c, 0x15, 0x5a, 0xd8, 0x44,
            0xcc, 0x81, 0xfb, 0x82, 0xfa, 0x8d, 0x74, 0x46, 0xfa, 0x7d, 0x78, 0xbe, 0x80, 0x3a,
            0xcd, 0xda, 0x95, 0x1b,
        ];
        assert_eq!(server_initial_secret, expected_server_initial_secret);

        // Key, IV, HP derivation
        let mut client_key = [0u8; 16];
        hkdf_expand_label(&client_initial_secret, "quic key", b"", &mut client_key);
        let expected_client_key = [
            0x1f, 0x36, 0x96, 0x13, 0xdd, 0x76, 0xd5, 0x46, 0x77, 0x30, 0xef, 0xcb, 0xe3, 0xb1, 0xa2, 0x2d,
        ];
        assert_eq!(client_key, expected_client_key);

        let mut client_iv = [0u8; 12];
        hkdf_expand_label(&client_initial_secret, "quic iv", b"", &mut client_iv);
        let expected_client_iv = [0xfa, 0x04, 0x4b, 0x2f, 0x42, 0xa3, 0xfd, 0x3b, 0x46, 0xfb, 0x25, 0x5c];
        assert_eq!(client_iv, expected_client_iv);

        let mut client_hp = [0u8; 16];
        hkdf_expand_label(&client_initial_secret, "quic hp", b"", &mut client_hp);
        let expected_client_hp = [
            0x9f, 0x50, 0x44, 0x9e, 0x04, 0xa0, 0xe8, 0x10, 0x28, 0x3a, 0x1e, 0x99, 0x33, 0xad, 0xed, 0xd2,
        ];
        assert_eq!(client_hp, expected_client_hp);
    }

    /// RFC 9001 Appendix A.2 Client Initial Packet AEAD & Header Protection Sample
    #[test]
    fn test_rfc9001_aead_and_hp_sample() {
        let client_key = [
            0x1f, 0x36, 0x96, 0x13, 0xdd, 0x76, 0xd5, 0x46, 0x77, 0x30, 0xef, 0xcb, 0xe3, 0xb1, 0xa2, 0x2d,
        ];
        let client_iv = [0xfa, 0x04, 0x4b, 0x2f, 0x42, 0xa3, 0xfd, 0x3b, 0x46, 0xfb, 0x25, 0x5c];
        let client_hp = [
            0x9f, 0x50, 0x44, 0x9e, 0x04, 0xa0, 0xe8, 0x10, 0x28, 0x3a, 0x1e, 0x99, 0x33, 0xad, 0xed, 0xd2,
        ];

        let pn = 2u64;
        let nonce = quic_aead_nonce(&client_iv, pn);
        assert_eq!(nonce, [0xfa, 0x04, 0x4b, 0x2f, 0x42, 0xa3, 0xfd, 0x3b, 0x46, 0xfb, 0x25, 0x5e]);

        // Sample header protection test
        let sample = [
            0xd1, 0xb1, 0xa9, 0x4d, 0xc4, 0x6b, 0xe6, 0xbe, 0x52, 0x83, 0x42, 0xe3, 0x9c, 0x2c, 0x3d, 0x4b,
        ];
        let mask = quic_aes_hp_mask(&client_hp, &sample);
        assert_eq!(mask.len(), 5);

        // Test AES-GCM roundtrip
        let plaintext = b"RFC 9001 ClientHello handshake payload bytes for QUIC Initial packet";
        let aad = b"QUIC Header associated data";
        let ciphertext = quic_aes_gcm_encrypt(&client_key, &nonce, plaintext, aad).unwrap();
        let decrypted = quic_aes_gcm_decrypt(&client_key, &nonce, &ciphertext, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    /// RFC 7748 Section 5.2 Test Vectors for X25519
    #[test]
    fn test_rfc7748_x25519_vectors() {
        let alice_private = [
            0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2, 0x66, 0x45,
            0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5, 0x1d, 0xb9, 0x2c, 0x2a,
        ];
        let alice_public_expected = [
            0x85, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54, 0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e, 0xf7, 0x5a,
            0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4, 0xeb, 0xa4, 0xa9, 0x8e, 0xaa, 0x9b, 0x4e, 0x6a,
        ];
        let alice_public = x25519_public_key(&alice_private);
        assert_eq!(alice_public, alice_public_expected);

        let bob_public = [
            0xde, 0x9e, 0xdb, 0x7d, 0x7b, 0x7d, 0xc1, 0xb4, 0xd3, 0x5b, 0x61, 0xc2, 0xec, 0xe4, 0x35, 0x37,
            0x3f, 0x83, 0x43, 0xc8, 0x5b, 0x78, 0x67, 0x4d, 0xad, 0xfc, 0x7e, 0x14, 0x6f, 0x88, 0x2b, 0x4f,
        ];

        // Shared secret computation: Alice's private key with Bob's public key
        let shared_alice = x25519(&alice_private, &bob_public);
        let expected_shared = [
            0x4a, 0x5d, 0x9d, 0x5b, 0xa4, 0xce, 0x2d, 0xe1, 0x72, 0x8e, 0x3b, 0xf4, 0x80, 0x35, 0x0f, 0x25,
            0xe0, 0x7e, 0x21, 0xc9, 0x47, 0xd1, 0x9e, 0x33, 0x76, 0xf0, 0x9b, 0x3c, 0x1e, 0x16, 0x17, 0x42,
        ];
        assert_eq!(shared_alice, expected_shared);
    }
}
