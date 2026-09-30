use bytes::{Buf, BufMut};
use pf_errors::{PlatformError, PlatformResult};

/// IETF QUIC Variable-Length Integer (RFC 9000 section 16).
/// Encodes integers up to 2^62-1 into 1, 2, 4, or 8 bytes based on the top 2 bits.
pub struct QuicVarInt;

impl QuicVarInt {
    pub const MAX: u64 = (1 << 62) - 1;

    /// Returns the number of bytes required to encode `value`.
    #[inline]
    pub fn encoded_len(value: u64) -> usize {
        if value < (1 << 6) {
            1
        } else if value < (1 << 14) {
            2
        } else if value < (1 << 30) {
            4
        } else if value <= Self::MAX {
            8
        } else {
            0
        }
    }

    /// Encodes a 62-bit integer into a buffer.
    pub fn encode<B: BufMut>(value: u64, buf: &mut B) -> PlatformResult<usize> {
        if value < (1 << 6) {
            buf.put_u8(value as u8);
            Ok(1)
        } else if value < (1 << 14) {
            buf.put_u16((0b01 << 14) | (value as u16));
            Ok(2)
        } else if value < (1 << 30) {
            buf.put_u32((0b10 << 30) | (value as u32));
            Ok(4)
        } else if value <= Self::MAX {
            buf.put_u64((0b11 << 62) | value);
            Ok(8)
        } else {
            Err(PlatformError::Validation(
                "Value exceeds QUIC 62-bit limit".into(),
            ))
        }
    }

    /// Decodes a 62-bit integer from a buffer.
    pub fn decode<B: Buf>(buf: &mut B) -> PlatformResult<u64> {
        if !buf.has_remaining() {
            return Err(PlatformError::Codec(
                "Buffer underflow reading QUIC varint".into(),
            ));
        }

        let first = buf.chunk()[0];
        let tag = (first & 0b1100_0000) >> 6;

        match tag {
            0b00 => {
                let v = buf.get_u8() & 0b0011_1111;
                Ok(v as u64)
            }
            0b01 => {
                if buf.remaining() < 2 {
                    return Err(PlatformError::Codec(
                        "Underflow reading 2-byte QUIC varint".into(),
                    ));
                }
                let v = buf.get_u16() & 0x3FFF;
                Ok(v as u64)
            }
            0b10 => {
                if buf.remaining() < 4 {
                    return Err(PlatformError::Codec(
                        "Underflow reading 4-byte QUIC varint".into(),
                    ));
                }
                let v = buf.get_u32() & 0x3FFF_FFFF;
                Ok(v as u64)
            }
            0b11 => {
                if buf.remaining() < 8 {
                    return Err(PlatformError::Codec(
                        "Underflow reading 8-byte QUIC varint".into(),
                    ));
                }
                let v = buf.get_u64() & 0x3FFF_FFFF_FFFF_FFFF;
                Ok(v)
            }
            _ => unreachable!(),
        }
    }
}

/// Standard Unsigned LEB128 Variable-Length Integer Encoding.
pub struct ULEB128;

impl ULEB128 {
    pub fn encode<B: BufMut>(mut value: u64, buf: &mut B) -> usize {
        let mut bytes_written = 0;
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            buf.put_u8(byte);
            bytes_written += 1;
            if value == 0 {
                break;
            }
        }
        bytes_written
    }

    pub fn decode<B: Buf>(buf: &mut B) -> PlatformResult<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;
        while buf.has_remaining() {
            let byte = buf.get_u8();
            result |= ((byte & 0x7F) as u64) << shift;
            if (byte & 0x80) == 0 {
                return Ok(result);
            }
            shift += 7;
            if shift >= 64 {
                return Err(PlatformError::Codec("ULEB128 overflow".into()));
            }
        }
        Err(PlatformError::Codec("ULEB128 unexpected EOF".into()))
    }
}

/// Bit-level stream reader.
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_offset: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            bit_offset: 0,
        }
    }

    pub fn read_bit(&mut self) -> Option<bool> {
        let byte_idx = self.bit_offset / 8;
        let bit_idx = 7 - (self.bit_offset % 8);
        if byte_idx >= self.data.len() {
            return None;
        }
        self.bit_offset += 1;
        Some((self.data[byte_idx] & (1 << bit_idx)) != 0)
    }

    pub fn read_bits(&mut self, n: usize) -> Option<u64> {
        if n > 64 {
            return None;
        }
        let mut result = 0;
        for _ in 0..n {
            let bit = self.read_bit()?;
            result = (result << 1) | (bit as u64);
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quic_varint_roundtrip() {
        let test_values = [
            0,
            1,
            63,
            64,
            16383,
            16384,
            1073741823,
            1073741824,
            QuicVarInt::MAX,
        ];
        for &val in &test_values {
            let mut buf = Vec::new();
            let written = QuicVarInt::encode(val, &mut buf).unwrap();
            assert_eq!(written, QuicVarInt::encoded_len(val));
            let mut cursor = &buf[..];
            let decoded = QuicVarInt::decode(&mut cursor).unwrap();
            assert_eq!(decoded, val);
        }
    }

    #[test]
    fn test_uleb128_roundtrip() {
        let test_values = [0, 1, 127, 128, 255, 16384, 1000000, u64::MAX];
        for &val in &test_values {
            let mut buf = Vec::new();
            ULEB128::encode(val, &mut buf);
            let mut cursor = &buf[..];
            let decoded = ULEB128::decode(&mut cursor).unwrap();
            assert_eq!(decoded, val);
        }
    }

    #[test]
    fn test_bit_reader() {
        let data = [0b1011_0001, 0b1100_1010];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bits(5), Some(0b10001));
    }
}
