use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use memmap2::Mmap;
use serde::{Serialize, Deserialize};
use crate::lsm::bloom::BloomFilter;
use crate::{Result, VectorLawDBError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSTableMetadata {
    pub num_entries: usize,
    pub min_key: Vec<u8>,
    pub max_key: Vec<u8>,
    pub created_at: u64,
    pub level: u8,
}

pub struct SSTable {
    path: PathBuf,
    metadata: SSTableMetadata,
    bloom_filter: BloomFilter,
    index: Vec<(Vec<u8>, u64)>, // (key, offset)
    mmap: Option<Mmap>,
}

impl SSTable {
    /// Create SSTable from MemTable
    pub fn create_from_memtable<P: AsRef<Path>>(
        memtable: &super::MemTable,
        path: P,
        level: u8,
    ) -> Result<Self> {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let entries = memtable.iter();

        // Sort entries
        let mut sorted_entries = entries;
        sorted_entries.sort_by(|a, b| a.key.cmp(&b.key));

        // Write header (reserve 256 bytes)
        file.write_all(&[0u8; 256])?;

        // Write entries and build index
        let mut index = Vec::new();
        let mut bloom = BloomFilter::new(sorted_entries.len() * 10, 3);

        for entry in &sorted_entries {
            let offset = file.stream_position()?;

            // Write entry: [key_len][key][value_len][value][timestamp][deleted]
            let key_len = entry.key.len() as u32;
            let value_len = entry.value.len() as u32;

            file.write_all(&key_len.to_le_bytes())?;
            file.write_all(&entry.key)?;
            file.write_all(&value_len.to_le_bytes())?;
            file.write_all(&entry.value)?;
            file.write_all(&entry.timestamp.to_le_bytes())?;
            file.write_all(&[entry.deleted as u8])?;

            // Add to index
            index.push((entry.key.clone(), offset));

            // Add to bloom filter
            bloom.add(&entry.key);
        }

        // Write index block
        let index_offset = file.stream_position()?;
        for (key, offset) in &index {
            let key_len = key.len() as u32;
            file.write_all(&key_len.to_le_bytes())?;
            file.write_all(key)?;
            file.write_all(&offset.to_le_bytes())?;
        }

        // Write bloom filter
        let bloom_offset = file.stream_position()?;
        let bloom_bytes = bincode::serialize(&bloom)
            .map_err(|e| VectorLawDBError::Serialization(e.to_string()))?;
        file.write_all(&bloom_bytes)?;

        // Write footer
        let _footer_offset = file.stream_position()?;
        file.write_all(&index_offset.to_le_bytes())?;
        file.write_all(&bloom_offset.to_le_bytes())?;
        file.write_all(&(index.len() as u64).to_le_bytes())?;

        // Write metadata to header
        let min_key = sorted_entries.first().map(|e| e.key.clone()).unwrap_or_default();
        let max_key = sorted_entries.last().map(|e| e.key.clone()).unwrap_or_default();

        let metadata = SSTableMetadata {
            num_entries: sorted_entries.len(),
            min_key,
            max_key,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            level,
        };

        let metadata_bytes = bincode::serialize(&metadata)
            .map_err(|e| VectorLawDBError::Serialization(e.to_string()))?;

        file.seek(SeekFrom::Start(0))?;
        file.write_all(&metadata_bytes)?;

        file.flush()?;
        drop(file);

        // Memory-map the file
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(Self {
            path: path.to_path_buf(),
            metadata,
            bloom_filter: bloom,
            index,
            mmap: Some(mmap),
        })
    }

    /// Open existing SSTable
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path)?;

        // Read metadata from header
        let mut metadata_buf = vec![0u8; 256];
        file.read_exact(&mut metadata_buf)?;

        let metadata: SSTableMetadata = bincode::deserialize(&metadata_buf)
            .map_err(|e| VectorLawDBError::Serialization(e.to_string()))?;

        // Read footer to find offsets
        file.seek(SeekFrom::End(-24))?;
        let mut footer = [0u8; 24];
        file.read_exact(&mut footer)?;

        let index_offset = u64::from_le_bytes(footer[0..8].try_into().unwrap());
        let bloom_offset = u64::from_le_bytes(footer[8..16].try_into().unwrap());
        let num_entries = u64::from_le_bytes(footer[16..24].try_into().unwrap()) as usize;

        // Read index
        file.seek(SeekFrom::Start(index_offset))?;
        let mut index = Vec::with_capacity(num_entries);

        for _ in 0..num_entries {
            let mut key_len_buf = [0u8; 4];
            file.read_exact(&mut key_len_buf)?;
            let key_len = u32::from_le_bytes(key_len_buf) as usize;

            let mut key = vec![0u8; key_len];
            file.read_exact(&mut key)?;

            let mut offset_buf = [0u8; 8];
            file.read_exact(&mut offset_buf)?;
            let offset = u64::from_le_bytes(offset_buf);

            index.push((key, offset));
        }

        // Read bloom filter
        file.seek(SeekFrom::Start(bloom_offset))?;
        let bloom_size = index_offset - bloom_offset;
        let mut bloom_bytes = vec![0u8; bloom_size as usize];
        file.read_exact(&mut bloom_bytes)?;

        let bloom_filter: BloomFilter = bincode::deserialize(&bloom_bytes)
            .map_err(|e| VectorLawDBError::Serialization(e.to_string()))?;

        // Memory-map
        drop(file);
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(Self {
            path: path.to_path_buf(),
            metadata,
            bloom_filter,
            index,
            mmap: Some(mmap),
        })
    }

    /// Get value for key
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // Check bloom filter first
        if !self.bloom_filter.might_contain(key) {
            return Ok(None);
        }

        // Binary search in index
        let pos = self.index.binary_search_by_key(&key, |(k, _)| k.as_slice());

        let offset = match pos {
            Ok(idx) => self.index[idx].1,
            Err(_) => return Ok(None),
        };

        // Read entry from mmap
        let mmap = self.mmap.as_ref().unwrap();
        let mut cursor = offset as usize;

        // Read key length
        let key_len = u32::from_le_bytes(
            mmap[cursor..cursor + 4].try_into().unwrap()
        ) as usize;
        cursor += 4;

        // Skip key (we already know it matches)
        cursor += key_len;

        // Read value length
        let value_len = u32::from_le_bytes(
            mmap[cursor..cursor + 4].try_into().unwrap()
        ) as usize;
        cursor += 4;

        // Read value
        let value = mmap[cursor..cursor + value_len].to_vec();
        cursor += value_len;

        // Read timestamp
        let _timestamp = u64::from_le_bytes(
            mmap[cursor..cursor + 8].try_into().unwrap()
        );
        cursor += 8;

        // Read deleted flag
        let deleted = mmap[cursor] != 0;

        if deleted {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub fn metadata(&self) -> &SSTableMetadata {
        &self.metadata
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
