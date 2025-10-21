use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use crate::Result;

/// Tombstone marker for deletions
const TOMBSTONE: &[u8] = b"__TOMBSTONE__";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub timestamp: u64,
    pub deleted: bool,
}

pub struct MemTable {
    data: Arc<RwLock<BTreeMap<Vec<u8>, Entry>>>,
    size_bytes: Arc<parking_lot::Mutex<usize>>,
    max_size_bytes: usize,
    created_at: std::time::Instant,
}

impl MemTable {
    pub fn new(max_size_mb: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(BTreeMap::new())),
            size_bytes: Arc::new(parking_lot::Mutex::new(0)),
            max_size_bytes: max_size_mb * 1024 * 1024,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn put(&self, key: Vec<u8>, value: Vec<u8>, timestamp: u64) -> Result<()> {
        let mut data = self.data.write().unwrap();
        let mut size = self.size_bytes.lock();

        let entry = Entry {
            key: key.clone(),
            value,
            timestamp,
            deleted: false,
        };

        // Calculate size delta
        let old_size = data.get(&key).map(|e| e.key.len() + e.value.len()).unwrap_or(0);
        let new_size = entry.key.len() + entry.value.len();

        data.insert(key, entry);
        *size = *size - old_size + new_size;

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Option<Entry> {
        let data = self.data.read().unwrap();
        data.get(key).filter(|e| !e.deleted).cloned()
    }

    pub fn delete(&self, key: Vec<u8>, timestamp: u64) -> Result<()> {
        let mut data = self.data.write().unwrap();

        let entry = Entry {
            key: key.clone(),
            value: TOMBSTONE.to_vec(),
            timestamp,
            deleted: true,
        };

        data.insert(key, entry);
        Ok(())
    }

    pub fn is_full(&self) -> bool {
        *self.size_bytes.lock() >= self.max_size_bytes
    }

    pub fn size_bytes(&self) -> usize {
        *self.size_bytes.lock()
    }

    pub fn iter(&self) -> Vec<Entry> {
        let data = self.data.read().unwrap();
        data.values().cloned().collect()
    }

    pub fn clear(&self) {
        let mut data = self.data.write().unwrap();
        let mut size = self.size_bytes.lock();

        data.clear();
        *size = 0;
    }
}

// Thread-safe clone
impl Clone for MemTable {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            size_bytes: Arc::clone(&self.size_bytes),
            max_size_bytes: self.max_size_bytes,
            created_at: self.created_at,
        }
    }
}
