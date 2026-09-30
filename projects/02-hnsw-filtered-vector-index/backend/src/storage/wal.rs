//! Write-Ahead Log for persistent vector operations

use serde::{Serialize, Deserialize};
use std::fs::{OpenOptions, File};
use std::io::{Write, Read, BufWriter, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalRecord {
    Insert { id: u64, vector: Vec<f32>, metadata: Vec<String> },
    Delete { id: u64 },
}

pub struct VectorWal {
    writer: BufWriter<File>,
}

impl VectorWal {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    pub fn append(&mut self, record: &WalRecord) -> std::io::Result<()> {
        let bytes = serde_json::to_vec(record)?;
        let len = (bytes.len() as u32).to_le_bytes();
        self.writer.write_all(&len)?;
        self.writer.write_all(&bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn replay<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<WalRecord>> {
        if !path.as_ref().exists() {
            return Ok(Vec::new());
        }
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut records = Vec::new();
        let mut len_buf = [0u8; 4];

        while reader.read_exact(&mut len_buf).is_ok() {
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut record_buf = vec![0u8; len];
            reader.read_exact(&mut record_buf)?;
            let record: WalRecord = serde_json::from_slice(&record_buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            records.push(record);
        }

        Ok(records)
    }
}
