use serde::{Serialize, Deserialize};
use ahash::AHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomFilter {
    bits: Vec<u8>,
    size: usize,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn new(size: usize, num_hashes: usize) -> Self {
        let byte_size = (size + 7) / 8;
        Self {
            bits: vec![0u8; byte_size],
            size,
            num_hashes,
        }
    }

    pub fn add(&mut self, key: &[u8]) {
        for i in 0..self.num_hashes {
            let hash = self.hash(key, i);
            let bit_pos = (hash % self.size as u64) as usize;
            let byte_pos = bit_pos / 8;
            let bit_offset = bit_pos % 8;

            self.bits[byte_pos] |= 1 << bit_offset;
        }
    }

    pub fn might_contain(&self, key: &[u8]) -> bool {
        for i in 0..self.num_hashes {
            let hash = self.hash(key, i);
            let bit_pos = (hash % self.size as u64) as usize;
            let byte_pos = bit_pos / 8;
            let bit_offset = bit_pos % 8;

            if (self.bits[byte_pos] & (1 << bit_offset)) == 0 {
                return false;
            }
        }
        true
    }

    fn hash(&self, key: &[u8], seed: usize) -> u64 {
        let mut hasher = AHasher::default();
        seed.hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }
}
