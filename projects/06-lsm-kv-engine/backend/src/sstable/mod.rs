//! SSTable File Format, Block Index, Bloom Filter & Block Cache Integration (06-040..048, 06-066..074)

use bytes::{BufMut, BytesMut};
use pf_checksum::crc32;
use pf_collections::BloomFilter;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockIndexEntry {
    pub first_key: Vec<u8>,
    pub last_key: Vec<u8>,
    pub offset: u64,
    pub length: u32,
}

pub struct SsTableBuilder {
    path: PathBuf,
    entries: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    bloom: BloomFilter,
    #[allow(dead_code)]
    estimated_entries: usize,
}

impl SsTableBuilder {
    pub fn new<P: AsRef<Path>>(path: P, estimated_entries: usize) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            entries: BTreeMap::new(),
            bloom: BloomFilter::with_capacity(std::cmp::max(100, estimated_entries), 0.01),
            estimated_entries,
        }
    }

    pub fn add(&mut self, key: &[u8], value: Option<&[u8]>) {
        self.bloom.insert(key);
        self.entries.insert(key.to_vec(), value.map(|v| v.to_vec()));
    }

    pub fn finish(self) -> std::io::Result<SsTableReader> {
        let mut file = File::create(&self.path)?;
        let mut block_index = Vec::new();
        let mut current_block = BytesMut::new();
        let mut block_first_key: Option<Vec<u8>> = None;
        let mut block_last_key: Option<Vec<u8>> = None;
        let mut current_offset = 0u64;

        let block_size_threshold = 4096;

        for (k, v) in self.entries {
            if block_first_key.is_none() {
                block_first_key = Some(k.clone());
            }
            block_last_key = Some(k.clone());

            let k_len = k.len() as u32;
            let v_len = v.as_ref().map(|val| val.len()).unwrap_or(0) as u32;
            let is_tombstone = if v.is_none() { 1u8 } else { 0u8 };

            current_block.put_u32(k_len);
            current_block.put_slice(&k);
            current_block.put_u8(is_tombstone);
            current_block.put_u32(v_len);
            if let Some(val) = v {
                current_block.put_slice(&val);
            }

            if current_block.len() >= block_size_threshold {
                let checksum = crc32(&current_block);
                file.write_all(&checksum.to_be_bytes())?;
                file.write_all(&(current_block.len() as u32).to_be_bytes())?;
                file.write_all(&current_block)?;

                let block_len = (8 + current_block.len()) as u32;
                block_index.push(BlockIndexEntry {
                    first_key: block_first_key.take().unwrap(),
                    last_key: block_last_key.take().unwrap(),
                    offset: current_offset,
                    length: block_len,
                });

                current_offset += block_len as u64;
                current_block.clear();
            }
        }

        if !current_block.is_empty() {
            let checksum = crc32(&current_block);
            file.write_all(&checksum.to_be_bytes())?;
            file.write_all(&(current_block.len() as u32).to_be_bytes())?;
            file.write_all(&current_block)?;

            let block_len = (8 + current_block.len()) as u32;
            block_index.push(BlockIndexEntry {
                first_key: block_first_key.unwrap(),
                last_key: block_last_key.unwrap(),
                offset: current_offset,
                length: block_len,
            });
            current_offset += block_len as u64;
        }

        // Write Bloom Filter & Index Offset Footer
        let index_offset = current_offset;
        let mut index_buf = BytesMut::new();
        index_buf.put_u32(block_index.len() as u32);
        for entry in &block_index {
            index_buf.put_u32(entry.first_key.len() as u32);
            index_buf.put_slice(&entry.first_key);
            index_buf.put_u32(entry.last_key.len() as u32);
            index_buf.put_slice(&entry.last_key);
            index_buf.put_u64(entry.offset);
            index_buf.put_u32(entry.length);
        }
        file.write_all(&index_buf)?;

        // Footer: index_offset (8B) + magic (4B: 0x4C534D31 = "LSM1")
        file.write_all(&index_offset.to_be_bytes())?;
        file.write_all(&0x4C534D31u32.to_be_bytes())?;
        file.sync_data()?;

        Ok(SsTableReader {
            path: self.path,
            bloom: self.bloom,
            index: block_index,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SsTableReader {
    pub path: PathBuf,
    pub bloom: BloomFilter,
    pub index: Vec<BlockIndexEntry>,
}

impl SsTableReader {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let file_len = file.metadata()?.len();

        if file_len < 12 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "SSTable too small",
            ));
        }

        // Read footer
        file.seek(SeekFrom::End(-12))?;
        let mut footer = [0u8; 12];
        file.read_exact(&mut footer)?;

        let index_offset = u64::from_be_bytes(footer[..8].try_into().unwrap());
        let magic = u32::from_be_bytes(footer[8..12].try_into().unwrap());
        if magic != 0x4C534D31 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid SSTable magic",
            ));
        }

        // Read index
        file.seek(SeekFrom::Start(index_offset))?;
        let mut index_len_buf = [0u8; 4];
        file.read_exact(&mut index_len_buf)?;
        let num_blocks = u32::from_be_bytes(index_len_buf) as usize;

        let mut index = Vec::with_capacity(num_blocks);
        for _ in 0..num_blocks {
            let mut k1_len_buf = [0u8; 4];
            file.read_exact(&mut k1_len_buf)?;
            let k1_len = u32::from_be_bytes(k1_len_buf) as usize;
            let mut k1 = vec![0u8; k1_len];
            file.read_exact(&mut k1)?;

            let mut k2_len_buf = [0u8; 4];
            file.read_exact(&mut k2_len_buf)?;
            let k2_len = u32::from_be_bytes(k2_len_buf) as usize;
            let mut k2 = vec![0u8; k2_len];
            file.read_exact(&mut k2)?;

            let mut meta_buf = [0u8; 12];
            file.read_exact(&mut meta_buf)?;
            let offset = u64::from_be_bytes(meta_buf[..8].try_into().unwrap());
            let length = u32::from_be_bytes(meta_buf[8..12].try_into().unwrap());

            index.push(BlockIndexEntry {
                first_key: k1,
                last_key: k2,
                offset,
                length,
            });
        }

        let mut reader = Self {
            path,
            bloom: BloomFilter::with_capacity(std::cmp::max(100, num_blocks * 100), 0.01),
            index,
        };
        let entries = reader.scan_all()?;
        for (k, _) in entries {
            reader.bloom.insert(&k);
        }
        Ok(reader)
    }

    pub fn get(&self, key: &[u8]) -> std::io::Result<Option<Option<Vec<u8>>>> {
        // Fast path: Bloom filter check
        if !self.bloom.contains(key) {
            return Ok(None); // Definitely not present
        }

        // Locate block in index
        let candidate_block = self
            .index
            .iter()
            .find(|entry| key >= entry.first_key.as_slice() && key <= entry.last_key.as_slice());

        if let Some(entry) = candidate_block {
            let mut file = File::open(&self.path)?;
            file.seek(SeekFrom::Start(entry.offset))?;

            let mut header = [0u8; 8];
            file.read_exact(&mut header)?;
            let expected_crc = u32::from_be_bytes(header[..4].try_into().unwrap());
            let block_payload_len = u32::from_be_bytes(header[4..8].try_into().unwrap()) as usize;

            let mut block_data = vec![0u8; block_payload_len];
            file.read_exact(&mut block_data)?;

            if crc32(&block_data) != expected_crc {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "SSTable block CRC mismatch",
                ));
            }

            let mut cursor = 0;
            while cursor + 9 <= block_data.len() {
                let k_len =
                    u32::from_be_bytes(block_data[cursor..cursor + 4].try_into().unwrap()) as usize;
                cursor += 4;
                let k = &block_data[cursor..cursor + k_len];
                cursor += k_len;
                let is_tombstone = block_data[cursor] == 1;
                cursor += 1;
                let v_len =
                    u32::from_be_bytes(block_data[cursor..cursor + 4].try_into().unwrap()) as usize;
                cursor += 4;
                let val = if is_tombstone {
                    None
                } else {
                    Some(block_data[cursor..cursor + v_len].to_vec())
                };
                cursor += v_len;

                if k == key {
                    return Ok(Some(val));
                }
            }
        }

        Ok(None)
    }

    #[allow(clippy::type_complexity)]
    pub fn scan_all(&self) -> std::io::Result<Vec<(Vec<u8>, Option<Vec<u8>>)>> {
        let mut file = File::open(&self.path)?;
        let mut entries = Vec::new();

        for entry in &self.index {
            file.seek(SeekFrom::Start(entry.offset))?;
            let mut header = [0u8; 8];
            file.read_exact(&mut header)?;
            let expected_crc = u32::from_be_bytes(header[..4].try_into().unwrap());
            let block_payload_len = u32::from_be_bytes(header[4..8].try_into().unwrap()) as usize;

            let mut block_data = vec![0u8; block_payload_len];
            file.read_exact(&mut block_data)?;

            if crc32(&block_data) != expected_crc {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "SSTable block CRC mismatch",
                ));
            }

            let mut cursor = 0;
            while cursor + 9 <= block_data.len() {
                let k_len =
                    u32::from_be_bytes(block_data[cursor..cursor + 4].try_into().unwrap()) as usize;
                cursor += 4;
                if cursor + k_len + 5 > block_data.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "SSTable corrupt block key length",
                    ));
                }
                let k = block_data[cursor..cursor + k_len].to_vec();
                cursor += k_len;
                let is_tombstone = block_data[cursor] == 1;
                cursor += 1;
                let v_len =
                    u32::from_be_bytes(block_data[cursor..cursor + 4].try_into().unwrap()) as usize;
                cursor += 4;
                if !is_tombstone && cursor + v_len > block_data.len() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "SSTable corrupt block value length",
                    ));
                }
                let val = if is_tombstone {
                    None
                } else {
                    let v = block_data[cursor..cursor + v_len].to_vec();
                    cursor += v_len;
                    Some(v)
                };

                entries.push((k, val));
            }
        }

        Ok(entries)
    }
}
