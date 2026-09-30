use pf_checksum::xxhash64;

/// Space-efficient Bloom Filter using double-hashing (Kirsch-Mitzenmacher technique).
#[derive(Clone, Debug)]
pub struct BloomFilter {
    bits: Vec<u64>,
    num_bits: usize,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn with_capacity(expected_items: usize, false_positive_rate: f64) -> Self {
        let ln2_sq = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        let num_bits =
            (-(expected_items as f64) * false_positive_rate.ln() / ln2_sq).ceil() as usize;
        let num_bits = std::cmp::max(64, num_bits);
        let num_hashes =
            ((num_bits as f64 / expected_items as f64) * std::f64::consts::LN_2).ceil() as usize;
        let num_u64s = num_bits.div_ceil(64);

        Self {
            bits: vec![0u64; num_u64s],
            num_bits,
            num_hashes: std::cmp::max(1, num_hashes),
        }
    }

    pub fn insert(&mut self, item: &[u8]) {
        let h1 = xxhash64(item, 0);
        let h2 = xxhash64(item, 0x9E3779B97F4A7C15);

        for i in 0..self.num_hashes {
            let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
            let bit_idx = (combined as usize) % self.num_bits;
            self.bits[bit_idx / 64] |= 1u64 << (bit_idx % 64);
        }
    }

    pub fn contains(&self, item: &[u8]) -> bool {
        let h1 = xxhash64(item, 0);
        let h2 = xxhash64(item, 0x9E3779B97F4A7C15);

        for i in 0..self.num_hashes {
            let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
            let bit_idx = (combined as usize) % self.num_bits;
            if (self.bits[bit_idx / 64] & (1u64 << (bit_idx % 64))) == 0 {
                return false;
            }
        }
        true
    }

    pub fn clear(&mut self) {
        self.bits.fill(0);
    }
}

/// Fixed-capacity circular ring buffer with zero heap reallocation.
#[derive(Clone, Debug)]
pub struct RingBuffer<T, const N: usize> {
    buffer: [Option<T>; N],
    head: usize,
    len: usize,
}

impl<T: Clone, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            buffer: std::array::from_fn(|_| None),
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, item: T) -> Option<T> {
        let idx = (self.head + self.len) % N;
        let old = self.buffer[idx].replace(item);
        if self.len < N {
            self.len += 1;
        } else {
            self.head = (self.head + 1) % N;
        }
        old
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            None
        } else {
            let actual_idx = (self.head + index) % N;
            self.buffer[actual_idx].as_ref()
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        N
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter() {
        let mut bf = BloomFilter::with_capacity(1000, 0.01);
        bf.insert(b"user_12345");
        bf.insert(b"transaction_98765");

        assert!(bf.contains(b"user_12345"));
        assert!(bf.contains(b"transaction_98765"));
        assert!(!bf.contains(b"non_existent_key"));
    }

    #[test]
    fn test_ring_buffer() {
        let mut ring: RingBuffer<u32, 3> = RingBuffer::new();
        ring.push(10);
        ring.push(20);
        ring.push(30);
        assert_eq!(ring.get(0), Some(&10));
        assert_eq!(ring.get(1), Some(&20));
        assert_eq!(ring.get(2), Some(&30));

        // Overwrite oldest (10)
        ring.push(40);
        assert_eq!(ring.get(0), Some(&20));
        assert_eq!(ring.get(1), Some(&30));
        assert_eq!(ring.get(2), Some(&40));
    }
}
