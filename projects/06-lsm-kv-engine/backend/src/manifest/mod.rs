use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::compaction::FileMetadata;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EditRecord {
    AddFile {
        level: usize,
        file_number: u64,
        size_bytes: u64,
        first_key: Vec<u8>,
        last_key: Vec<u8>,
    },
    DeleteFile {
        level: usize,
        file_number: u64,
    },
    LogNumber(u64),
    NextFileNumber(u64),
    LastSequenceNumber(u64),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VersionEdit {
    pub records: Vec<EditRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct Version {
    pub levels: Vec<Vec<FileMetadata>>,
    pub log_number: u64,
    pub next_file_number: u64,
    pub last_sequence_number: u64,
}

impl Version {
    pub fn apply_edit(&mut self, edit: &VersionEdit, db_dir: &Path) {
        for record in &edit.records {
            match record {
                EditRecord::AddFile {
                    level,
                    file_number,
                    size_bytes,
                    first_key,
                    last_key,
                } => {
                    while self.levels.len() <= *level {
                        self.levels.push(Vec::new());
                    }
                    let path = db_dir.join(format!("{:06}.sst", file_number));
                    self.levels[*level].push(FileMetadata {
                        file_number: *file_number,
                        level: *level,
                        path,
                        size_bytes: *size_bytes,
                        first_key: first_key.clone(),
                        last_key: last_key.clone(),
                    });
                }
                EditRecord::DeleteFile { level, file_number } => {
                    if *level < self.levels.len() {
                        self.levels[*level].retain(|f| f.file_number != *file_number);
                    }
                }
                EditRecord::LogNumber(num) => self.log_number = *num,
                EditRecord::NextFileNumber(num) => self.next_file_number = *num,
                EditRecord::LastSequenceNumber(num) => self.last_sequence_number = *num,
            }
        }
    }
}

pub struct ManifestManager {
    pub db_dir: PathBuf,
    pub manifest_file_number: u64,
    pub manifest_path: PathBuf,
}

impl ManifestManager {
    pub fn open(db_dir: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(db_dir)?;
        let current_path = db_dir.join("CURRENT");

        let manifest_num = if current_path.exists() {
            let manifest_name = fs::read_to_string(&current_path)?.trim().to_string();
            manifest_name
                .strip_prefix("MANIFEST-")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1)
        } else {
            1
        };

        let manifest_path = db_dir.join(format!("MANIFEST-{:06}", manifest_num));
        let manager = Self {
            db_dir: db_dir.to_path_buf(),
            manifest_file_number: manifest_num,
            manifest_path,
        };

        if !current_path.exists() {
            manager.set_current_pointer(manifest_num)?;
        }

        Ok(manager)
    }

    /// Sets the atomic CURRENT pointer to active manifest file.
    pub fn set_current_pointer(&self, manifest_num: u64) -> std::io::Result<()> {
        let manifest_name = format!("MANIFEST-{:06}\n", manifest_num);
        let tmp_path = self.db_dir.join("CURRENT.tmp");
        let current_path = self.db_dir.join("CURRENT");

        fs::write(&tmp_path, manifest_name.as_bytes())?;
        fs::rename(&tmp_path, &current_path)?;
        Ok(())
    }

    /// Appends a VersionEdit record to MANIFEST log with length-prefix and CRC32 checksum.
    pub fn log_edit(&self, edit: &VersionEdit) -> std::io::Result<()> {
        let serialized = serde_json::to_vec(edit)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.manifest_path)?;

        let checksum = pf_checksum::crc32(&serialized);

        file.write_all(&(serialized.len() as u32).to_le_bytes())?;
        file.write_all(&checksum.to_le_bytes())?;
        file.write_all(&serialized)?;
        file.sync_data()?;
        Ok(())
    }

    /// Replays the MANIFEST log on startup to reconstruct current Version state.
    pub fn recover(&self) -> std::io::Result<Version> {
        let mut version = Version::default();
        if !self.manifest_path.exists() {
            return Ok(version);
        }

        let mut file = File::open(&self.manifest_path)?;
        let mut buf_len = [0u8; 4];
        let mut buf_crc = [0u8; 4];

        while file.read_exact(&mut buf_len).is_ok() {
            file.read_exact(&mut buf_crc)?;
            let len = u32::from_le_bytes(buf_len) as usize;
            let expected_crc = u32::from_le_bytes(buf_crc);

            let mut record_buf = vec![0u8; len];
            file.read_exact(&mut record_buf)?;

            if pf_checksum::crc32(&record_buf) != expected_crc {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Manifest record CRC mismatch",
                ));
            }

            let edit: VersionEdit = serde_json::from_slice(&record_buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            version.apply_edit(&edit, &self.db_dir);
        }

        Ok(version)
    }
}
