/// Verifies an actual output hash against a known golden milestone checksum.
pub fn verify_golden_hash(label: &str, actual: &[u8], expected_hex: &str) {
    let actual_hash = blake3::hash(actual).to_hex().to_string();
    assert_eq!(
        actual_hash, expected_hex,
        "Golden milestone hash mismatch for '{}'! Expected: {}, Actual: {}",
        label, expected_hex, actual_hash
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_golden_verification() {
        let data = b"deterministic_test_stream_2026";
        let hash = blake3::hash(data).to_hex().to_string();
        verify_golden_hash("test_stream", data, &hash);
    }
}
