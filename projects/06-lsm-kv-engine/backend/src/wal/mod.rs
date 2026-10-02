//! Write-Ahead Log (WAL) with CRC32 Verification & Crash Recovery (06-014..022)

use bytes::{Buf, BufMut, BytesMut};
use pf_checksum::crc32;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WalOpType {
    Put = 1,
    Delete = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsyncPolicy {
    Always,
    Interval(usize),
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalRecord {
    pub seq_no: u64,
    pub op_type: WalOpType,
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
}

pub struct WalWriter {
    file: File,
    #[allow(dead_code)]
    path: PathBuf,
    current_seq_no: u64,
    fsync_policy: FsyncPolicy,
    sync_count: usize,
    unflushed_appends: usize,
}

impl WalWriter {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)?;

        Ok(Self {
            file,
            path,
            current_seq_no: 0,
            fsync_policy: FsyncPolicy::Always,
            sync_count: 0,
            unflushed_appends: 0,
        })
    }

    pub fn set_fsync_policy(&mut self, policy: FsyncPolicy) {
        self.fsync_policy = policy;
    }

    pub fn sync_count(&self) -> usize {
        self.sync_count
    }

    pub fn append(&mut self, record: &WalRecord, sync: bool) -> std::io::Result<u64> {
        self.current_seq_no = record.seq_no;

        let key_len = record.key.len() as u32;
        let val_len = record.value.as_ref().map(|v| v.len()).unwrap_or(0) as u32;

        let mut payload =
            BytesMut::with_capacity(8 + 1 + 4 + key_len as usize + 4 + val_len as usize);
        payload.put_u64(record.seq_no);
        payload.put_u8(record.op_type as u8);
        payload.put_u32(key_len);
        payload.put_slice(&record.key);
        payload.put_u32(val_len);
        if let Some(ref val) = record.value {
            payload.put_slice(val);
        }

        let checksum = crc32(&payload);
        let mut frame = BytesMut::with_capacity(4 + 4 + payload.len());
        frame.put_u32(checksum);
        frame.put_u32(payload.len() as u32);
        frame.put_slice(&payload);

        self.file.write_all(&frame)?;
        self.unflushed_appends += 1;

        let should_sync = sync
            || match self.fsync_policy {
                FsyncPolicy::Always => true,
                FsyncPolicy::Interval(n) if n > 0 => self.unflushed_appends >= n,
                _ => false,
            };

        if should_sync {
            self.sync_count += 1;
            self.unflushed_appends = 0;
            #[cfg(target_os = "macos")]
            {
                use std::os::unix::io::AsRawFd;
                unsafe {
                    libc::fcntl(self.file.as_raw_fd(), libc::F_FULLFSYNC);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                self.file.sync_data()?;
            }
        }

        Ok(self.current_seq_no)
    }

    pub fn recover<P: AsRef<Path>>(path: P) -> std::io::Result<(Vec<WalRecord>, u64)> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok((Vec::new(), 0));
        }

        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let mut cursor = 0;
        let mut records = Vec::new();
        let mut max_seq = 0;

        while cursor + 8 <= buffer.len() {
            let checksum = u32::from_be_bytes(buffer[cursor..cursor + 4].try_into().unwrap());
            let payload_len =
                u32::from_be_bytes(buffer[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
            cursor += 8;

            if cursor + payload_len > buffer.len() {
                // Incomplete tail write / truncated record during kill
                break;
            }

            let payload_bytes = &buffer[cursor..cursor + payload_len];
            let actual_checksum = crc32(payload_bytes);
            if actual_checksum != checksum {
                // Corrupted record detected
                break;
            }

            cursor += payload_len;

            // Decode payload
            let mut buf = payload_bytes;
            let seq_no = buf.get_u64();
            let op_code = buf.get_u8();
            let op_type = match op_code {
                1 => WalOpType::Put,
                2 => WalOpType::Delete,
                _ => break,
            };
            let k_len = buf.get_u32() as usize;
            let key = buf[..k_len].to_vec();
            buf.advance(k_len);

            let v_len = buf.get_u32() as usize;
            let value = match op_type {
                WalOpType::Put => {
                    let val = buf[..v_len].to_vec();
                    buf.advance(v_len);
                    Some(val)
                }
                WalOpType::Delete => None,
            };

            max_seq = std::cmp::max(max_seq, seq_no);
            records.push(WalRecord {
                seq_no,
                op_type,
                key,
                value,
            });
        }

        Ok((records, max_seq))
    }
}
