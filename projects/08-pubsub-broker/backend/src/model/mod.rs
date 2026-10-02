//! Protocol and Data Model (Features 08-001..08-013)

use bytes::{Buf, BufMut, Bytes, BytesMut};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 08-001..08-004: Core Record Structure with CRC32C validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub offset: u64,
    pub timestamp: i64,
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub headers: HashMap<String, String>,
    pub crc32: u32,
}

impl Record {
    pub fn new(offset: u64, timestamp: i64, key: Option<Vec<u8>>, value: Vec<u8>, headers: HashMap<String, String>) -> Self {
        let mut r = Self {
            offset,
            timestamp,
            key,
            value,
            headers,
            crc32: 0,
        };
        r.crc32 = r.compute_crc();
        r
    }

    pub fn compute_crc(&self) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&self.offset.to_be_bytes());
        hasher.update(&self.timestamp.to_be_bytes());
        if let Some(ref k) = self.key {
            hasher.update(k);
        }
        hasher.update(&self.value);
        for (k, v) in &self.headers {
            hasher.update(k.as_bytes());
            hasher.update(v.as_bytes());
        }
        hasher.finalize()
    }

    pub fn is_valid(&self) -> bool {
        self.compute_crc() == self.crc32
    }

    /// Binary wire encoding
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(64 + self.value.len());
        buf.put_u64(self.offset);
        buf.put_i64(self.timestamp);
        buf.put_u32(self.crc32);

        match &self.key {
            Some(k) => {
                buf.put_u32(k.len() as u32);
                buf.put_slice(k);
            }
            None => buf.put_u32(0xFFFF_FFFF),
        }

        buf.put_u32(self.value.len() as u32);
        buf.put_slice(&self.value);

        buf.put_u32(self.headers.len() as u32);
        for (k, v) in &self.headers {
            buf.put_u32(k.len() as u32);
            buf.put_slice(k.as_bytes());
            buf.put_u32(v.len() as u32);
            buf.put_slice(v.as_bytes());
        }

        buf.freeze()
    }

    /// Binary wire decoding
    pub fn decode(mut buf: Bytes) -> Result<Self, &'static str> {
        if buf.remaining() < 20 {
            return Err("Buffer too small for record header");
        }
        let offset = buf.get_u64();
        let timestamp = buf.get_i64();
        let crc32 = buf.get_u32();

        let key_len = buf.get_u32();
        let key = if key_len == 0xFFFF_FFFF {
            None
        } else {
            if buf.remaining() < key_len as usize {
                return Err("Buffer underflow for key");
            }
            let mut k = vec![0u8; key_len as usize];
            buf.copy_to_slice(&mut k);
            Some(k)
        };

        if buf.remaining() < 4 {
            return Err("Buffer underflow for value len");
        }
        let val_len = buf.get_u32() as usize;
        if buf.remaining() < val_len {
            return Err("Buffer underflow for value");
        }
        let mut value = vec![0u8; val_len];
        buf.copy_to_slice(&mut value);

        if buf.remaining() < 4 {
            return Err("Buffer underflow for headers count");
        }
        let header_count = buf.get_u32() as usize;
        let mut headers = HashMap::new();
        for _ in 0..header_count {
            if buf.remaining() < 4 {
                return Err("Buffer underflow for header key len");
            }
            let hk_len = buf.get_u32() as usize;
            if buf.remaining() < hk_len {
                return Err("Buffer underflow for header key");
            }
            let mut hk_bytes = vec![0u8; hk_len];
            buf.copy_to_slice(&mut hk_bytes);
            let hk = String::from_utf8(hk_bytes).map_err(|_| "Invalid UTF-8 header key")?;

            if buf.remaining() < 4 {
                return Err("Buffer underflow for header val len");
            }
            let hv_len = buf.get_u32() as usize;
            if buf.remaining() < hv_len {
                return Err("Buffer underflow for header val");
            }
            let mut hv_bytes = vec![0u8; hv_len];
            buf.copy_to_slice(&mut hv_bytes);
            let hv = String::from_utf8(hv_bytes).map_err(|_| "Invalid UTF-8 header val")?;

            headers.insert(hk, hv);
        }

        let record = Self {
            offset,
            timestamp,
            key,
            value,
            headers,
            crc32,
        };

        if !record.is_valid() {
            return Err("CRC32 mismatch in record");
        }

        Ok(record)
    }
}

/// 08-005..08-013: API Request/Response models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    pub topic: String,
    pub partition: Option<u32>,
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    pub topic: String,
    pub partition: u32,
    pub offset: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub topic: String,
    pub partition: u32,
    pub offset: u64,
    pub max_records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub records: Vec<Record>,
    pub high_watermark: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetCommitRequest {
    pub group_id: String,
    pub topic: String,
    pub partition: u32,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
