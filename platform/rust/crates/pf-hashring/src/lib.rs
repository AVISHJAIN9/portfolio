use pf_checksum::xxhash64;
use std::collections::BTreeMap;

/// Consistent Hash Ring (Ketama with Virtual Nodes).
pub struct HashRing<N: Clone> {
    vnodes_per_node: usize,
    ring: BTreeMap<u64, N>,
}

impl<N: Clone> HashRing<N> {
    pub fn new(vnodes_per_node: usize) -> Self {
        Self {
            vnodes_per_node: std::cmp::max(1, vnodes_per_node),
            ring: BTreeMap::new(),
        }
    }

    pub fn add_node(&mut self, node_id: &str, node: N) {
        for v in 0..self.vnodes_per_node {
            let key_str = format!("{}:{}", node_id, v);
            let hash = xxhash64(key_str.as_bytes(), 0);
            self.ring.insert(hash, node.clone());
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&N> {
        if self.ring.is_empty() {
            return None;
        }
        let hash = xxhash64(key, 0);
        // Find first node with hash >= key_hash (or wrap around to first node)
        if let Some((_, node)) = self.ring.range(hash..).next() {
            Some(node)
        } else {
            self.ring.values().next()
        }
    }
}

/// Jump Consistent Hashing (Lamping & Veach, 2014).
/// Fast, minimal memory, 0-allocation hash routing to `num_buckets`.
#[inline]
pub fn jump_consistent_hash(mut key: u64, num_buckets: i32) -> i32 {
    let mut b: i64 = -1;
    let mut j: i64 = 0;
    while j < num_buckets as i64 {
        b = j;
        key = key.wrapping_mul(2862933555777941757).wrapping_add(1);
        j = (((b + 1) as f64) * ((1i64 << 31) as f64) / (((key >> 33) + 1) as f64)) as i64;
    }
    b as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_ring() {
        let mut ring = HashRing::new(50);
        ring.add_node("node-1", "10.0.0.1:8080");
        ring.add_node("node-2", "10.0.0.2:8080");

        assert!(ring.get(b"user_100").is_some());
        assert!(ring.get(b"user_200").is_some());
    }

    #[test]
    fn test_jump_hash() {
        let bucket = jump_consistent_hash(123456789, 10);
        assert!((0..10).contains(&bucket));
    }
}
