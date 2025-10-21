pub mod memtable;
pub mod sstable;
pub mod wal;
pub mod bloom;
pub mod compaction;

pub use memtable::MemTable;
pub use sstable::SSTable;
pub use wal::WriteAheadLog;

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::{Result, VectorLawDBError};

pub struct LSMTree {
    data_dir: PathBuf,
    memtable: Arc<MemTable>,
    wal: Arc<WriteAheadLog>,
    levels: Arc<RwLock<Vec<Vec<Arc<SSTable>>>>>,
    next_timestamp: Arc<parking_lot::Mutex<u64>>,
}

impl LSMTree {
    pub fn new<P: Into<PathBuf>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)?;

        let wal_path = data_dir.join("wal.log");
        let wal = Arc::new(WriteAheadLog::new(wal_path)?);

        let memtable = Arc::new(MemTable::new(64)); // 64MB

        let mut tree = Self {
            data_dir,
            memtable: Arc::clone(&memtable),
            wal: Arc::clone(&wal),
            levels: Arc::new(RwLock::new(vec![Vec::new(); 6])), // L0-L5
            next_timestamp: Arc::new(parking_lot::Mutex::new(0)),
        };

        // Recovery from WAL
        tree.recover()?;

        Ok(tree)
    }

    fn get_timestamp(&self) -> u64 {
        let mut ts = self.next_timestamp.lock();
        let current = *ts;
        *ts += 1;
        current
    }

    pub fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let timestamp = self.get_timestamp();

        // Write to WAL first (durability)
        self.wal.append_put(key.clone(), value.clone(), timestamp)?;

        // Write to memtable
        self.memtable.put(key, value, timestamp)?;

        // Flush if full
        if self.memtable.is_full() {
            self.flush_memtable()?;
        }

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // Check memtable first
        if let Some(entry) = self.memtable.get(key) {
            return Ok(Some(entry.value));
        }

        // Check SSTables from newest to oldest
        let levels = self.levels.read();

        for level in levels.iter() {
            for sstable in level.iter().rev() {
                if let Some(value) = sstable.get(key)? {
                    return Ok(Some(value));
                }
            }
        }

        Ok(None)
    }

    pub fn delete(&self, key: Vec<u8>) -> Result<()> {
        let timestamp = self.get_timestamp();

        self.wal.append_delete(key.clone(), timestamp)?;
        self.memtable.delete(key, timestamp)?;

        if self.memtable.is_full() {
            self.flush_memtable()?;
        }

        Ok(())
    }

    fn flush_memtable(&self) -> Result<()> {
        let mut levels = self.levels.write();

        // Create SSTable from memtable
        let sstable_path = self.data_dir.join(format!("L0_{}.sst", levels[0].len()));
        let sstable = SSTable::create_from_memtable(&self.memtable, sstable_path, 0)?;

        levels[0].push(Arc::new(sstable));

        // Clear memtable and WAL
        self.memtable.clear();
        self.wal.truncate()?;

        // Trigger compaction if needed
        if levels[0].len() >= 4 {
            drop(levels);
            self.compact_level(0)?;
        }

        Ok(())
    }

    fn compact_level(&self, level: usize) -> Result<()> {
        // TODO: Implement leveled compaction
        tracing::info!("Compacting level {}", level);
        Ok(())
    }

    fn recover(&self) -> Result<()> {
        let entries = self.wal.replay()?;

        tracing::info!("Recovering {} WAL entries", entries.len());

        for entry in entries {
            match entry {
                wal::WALEntry::Put { key, value, timestamp } => {
                    self.memtable.put(key, value, timestamp)?;
                }
                wal::WALEntry::Delete { key, timestamp } => {
                    self.memtable.delete(key, timestamp)?;
                }
            }
        }

        Ok(())
    }
}
