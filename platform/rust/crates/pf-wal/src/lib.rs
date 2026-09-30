use pf_checksum::crc32;
use pf_errors::{PlatformError, PlatformResult};
use std::io::{Read, Seek, SeekFrom, Write};

/// In-Memory and Disk Segmented WAL Record Header:
/// [Magic: 2B (0x5741 'WA')] [CRC32: 4B] [Payload Len: 4B] [Sequence: 8B]
pub const WAL_MAGIC: u16 = 0x5741;
pub const HEADER_SIZE: usize = 18;

#[derive(Debug, Clone, PartialEq)]
pub struct WalRecord {
    pub sequence: u64,
    pub payload: Vec<u8>,
}

pub struct WalSegment<W: Write + Read + Seek> {
    writer: W,
    current_seq: u64,
}

impl<W: Write + Read + Seek> WalSegment<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            current_seq: 0,
        }
    }

    pub fn append(&mut self, payload: &[u8]) -> PlatformResult<u64> {
        self.current_seq += 1;
        let checksum = crc32(payload);

        let mut header = [0u8; HEADER_SIZE];
        header[0..2].copy_from_slice(&WAL_MAGIC.to_be_bytes());
        header[2..6].copy_from_slice(&checksum.to_be_bytes());
        header[6..10].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        header[10..18].copy_from_slice(&self.current_seq.to_be_bytes());

        self.writer
            .write_all(&header)
            .map_err(|e| PlatformError::Io(e.to_string()))?;
        self.writer
            .write_all(payload)
            .map_err(|e| PlatformError::Io(e.to_string()))?;
        self.writer
            .flush()
            .map_err(|e| PlatformError::Io(e.to_string()))?;

        Ok(self.current_seq)
    }

    pub fn recover_all(&mut self) -> PlatformResult<Vec<WalRecord>> {
        self.writer
            .seek(SeekFrom::Start(0))
            .map_err(|e| PlatformError::Io(e.to_string()))?;
        let mut records = Vec::new();

        loop {
            let mut header = [0u8; HEADER_SIZE];
            match self.writer.read_exact(&mut header) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(PlatformError::Io(e.to_string())),
            }

            let magic = u16::from_be_bytes([header[0], header[1]]);
            if magic != WAL_MAGIC {
                break; // Corrupted / Torn write boundary
            }

            let expected_crc = u32::from_be_bytes([header[2], header[3], header[4], header[5]]);
            let len = u32::from_be_bytes([header[6], header[7], header[8], header[9]]) as usize;
            let seq = u64::from_be_bytes(header[10..18].try_into().unwrap());

            let mut payload = vec![0u8; len];
            if self.writer.read_exact(&mut payload).is_err() {
                break; // Incomplete record
            }

            if crc32(&payload) != expected_crc {
                break; // Corrupted checksum
            }

            records.push(WalRecord {
                sequence: seq,
                payload,
            });
            self.current_seq = seq;
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_wal_write_and_recover() {
        let cursor = Cursor::new(Vec::new());
        let mut wal = WalSegment::new(cursor);

        wal.append(b"INSERT key_1 val_1").unwrap();
        wal.append(b"UPDATE key_1 val_2").unwrap();

        let recovered = wal.recover_all().unwrap();
        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered[0].sequence, 1);
        assert_eq!(recovered[0].payload, b"INSERT key_1 val_1");
        assert_eq!(recovered[1].sequence, 2);
        assert_eq!(recovered[1].payload, b"UPDATE key_1 val_2");
    }
}
