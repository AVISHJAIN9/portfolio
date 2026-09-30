use crc32fast::Hasher as Crc32Hasher;
use std::hash::Hasher;
use twox_hash::XxHash64;

/// Computes a standard hardware-accelerated CRC32 checksum.
#[inline]
pub fn crc32(data: &[u8]) -> u32 {
    let mut hasher = Crc32Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Computes 64-bit xxHash for ultra-fast non-cryptographic hashing.
#[inline]
pub fn xxhash64(data: &[u8], seed: u64) -> u64 {
    let mut hasher = XxHash64::with_seed(seed);
    hasher.write(data);
    hasher.finish()
}

/// Computes Adler-32 rolling checksum.
pub fn adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
}

/// Computes Blake3 256-bit cryptographic digest.
#[inline]
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksums() {
        let payload = b"PortfolioForge Deterministic Engine Payload 2026";
        assert_ne!(crc32(payload), 0);
        assert_ne!(xxhash64(payload, 0), 0);
        assert_ne!(adler32(payload), 0);
        assert_eq!(blake3_hash(payload).len(), 32);
    }
}
