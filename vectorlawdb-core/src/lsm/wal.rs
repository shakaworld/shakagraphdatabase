use std::fs::{File, OpenOptions};
use std::io::{self, Write, BufReader, Read};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use crate::{Result, VectorLawDBError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WALEntry {
    Put { key: Vec<u8>, value: Vec<u8>, timestamp: u64 },
    Delete { key: Vec<u8>, timestamp: u64 },
}

pub struct WriteAheadLog {
    path: PathBuf,
    file: parking_lot::Mutex<File>,
}

impl WriteAheadLog {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        Ok(Self {
            path,
            file: parking_lot::Mutex::new(file),
        })
    }

    pub fn append_put(&self, key: Vec<u8>, value: Vec<u8>, timestamp: u64) -> Result<()> {
        let entry = WALEntry::Put { key, value, timestamp };
        self.append_entry(&entry)
    }

    pub fn append_delete(&self, key: Vec<u8>, timestamp: u64) -> Result<()> {
        let entry = WALEntry::Delete { key, timestamp };
        self.append_entry(&entry)
    }

    fn append_entry(&self, entry: &WALEntry) -> Result<()> {
        let mut file = self.file.lock();

        // Serialize entry
        let data = bincode::serialize(entry)
            .map_err(|e| VectorLawDBError::Serialization(e.to_string()))?;

        // Write length + data + checksum
        let len = data.len() as u32;
        let checksum = crc32fast::hash(&data);

        file.write_all(&len.to_le_bytes())?;
        file.write_all(&data)?;
        file.write_all(&checksum.to_le_bytes())?;

        // Sync to disk (durability)
        file.sync_all()?;

        Ok(())
    }

    pub fn replay(&self) -> Result<Vec<WALEntry>> {
        let file = File::open(&self.path)?;
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();

        loop {
            // Read length
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf) {
                Ok(_) => {},
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }

            let len = u32::from_le_bytes(len_buf) as usize;

            // Read data
            let mut data = vec![0u8; len];
            reader.read_exact(&mut data)?;

            // Read checksum
            let mut checksum_buf = [0u8; 4];
            reader.read_exact(&mut checksum_buf)?;
            let expected_checksum = u32::from_le_bytes(checksum_buf);

            // Verify checksum
            let actual_checksum = crc32fast::hash(&data);
            if actual_checksum != expected_checksum {
                tracing::warn!("WAL entry checksum mismatch, stopping replay");
                break;
            }

            // Deserialize
            let entry: WALEntry = bincode::deserialize(&data)
                .map_err(|e| VectorLawDBError::Serialization(e.to_string()))?;

            entries.push(entry);
        }

        Ok(entries)
    }

    pub fn truncate(&self) -> Result<()> {
        let mut file = self.file.lock();
        file.set_len(0)?;
        file.sync_all()?;
        Ok(())
    }
}
