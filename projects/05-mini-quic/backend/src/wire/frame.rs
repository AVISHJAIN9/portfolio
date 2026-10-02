//! QUIC Frame Types (RFC 9000 Section 19)

use bytes::{Buf, BufMut, Bytes, BytesMut};
use pf_bytes::QuicVarInt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Padding {
        length: usize,
    },
    Ping,
    Ack(AckFrame),
    ResetStream {
        stream_id: u64,
        error_code: u64,
        final_size: u64,
    },
    StopSending {
        stream_id: u64,
        error_code: u64,
    },
    Crypto(CryptoFrame),
    NewToken {
        token: Bytes,
    },
    Stream(StreamFrame),
    MaxData {
        max_data: u64,
    },
    MaxStreamData {
        stream_id: u64,
        max_stream_data: u64,
    },
    MaxStreams {
        is_bidi: bool,
        max_streams: u64,
    },
    DataBlocked {
        data_limit: u64,
    },
    StreamDataBlocked {
        stream_id: u64,
        stream_data_limit: u64,
    },
    StreamsBlocked {
        is_bidi: bool,
        stream_limit: u64,
    },
    NewConnectionId {
        seq_num: u64,
        retire_prior_to: u64,
        connection_id: Vec<u8>,
        stateless_reset_token: [u8; 16],
    },
    RetireConnectionId {
        seq_num: u64,
    },
    ConnectionClose {
        error_code: u64,
        frame_type: Option<u64>,
        reason_phrase: String,
    },
    HandshakeDone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoFrame {
    pub offset: u64,
    pub data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamFrame {
    pub stream_id: u64,
    pub offset: u64,
    pub fin: bool,
    pub data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AckRange {
    pub gap: u64,
    pub ack_range_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AckFrame {
    pub largest_acknowledged: u64,
    pub ack_delay: u64,
    pub first_ack_range: u64,
    pub ack_ranges: Vec<AckRange>,
    pub ecn_counts: Option<EcnCounts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcnCounts {
    pub ect0: u64,
    pub ect1: u64,
    pub ce: u64,
}

impl Frame {
    pub fn encode(&self, dst: &mut BytesMut) {
        match self {
            Frame::Padding { length } => {
                dst.put_bytes(0x00, *length);
            }
            Frame::Ping => {
                let _ = QuicVarInt::encode(0x01, dst);
            }
            Frame::Ack(ack) => {
                let frame_type = if ack.ecn_counts.is_some() { 0x03 } else { 0x02 };
                let _ = QuicVarInt::encode(frame_type, dst);
                let _ = QuicVarInt::encode(ack.largest_acknowledged, dst);
                let _ = QuicVarInt::encode(ack.ack_delay, dst);
                let _ = QuicVarInt::encode(ack.ack_ranges.len() as u64, dst);
                let _ = QuicVarInt::encode(ack.first_ack_range, dst);
                for range in &ack.ack_ranges {
                    let _ = QuicVarInt::encode(range.gap, dst);
                    let _ = QuicVarInt::encode(range.ack_range_length, dst);
                }
                if let Some(ecn) = &ack.ecn_counts {
                    let _ = QuicVarInt::encode(ecn.ect0, dst);
                    let _ = QuicVarInt::encode(ecn.ect1, dst);
                    let _ = QuicVarInt::encode(ecn.ce, dst);
                }
            }
            Frame::Crypto(crypto) => {
                let _ = QuicVarInt::encode(0x06, dst);
                let _ = QuicVarInt::encode(crypto.offset, dst);
                let _ = QuicVarInt::encode(crypto.data.len() as u64, dst);
                dst.extend_from_slice(&crypto.data);
            }
            Frame::Stream(stream) => {
                let mut frame_type = 0x08;
                if stream.offset > 0 {
                    frame_type |= 0x04;
                }
                frame_type |= 0x02; // Always include explicit length
                if stream.fin {
                    frame_type |= 0x01;
                }
                let _ = QuicVarInt::encode(frame_type, dst);
                let _ = QuicVarInt::encode(stream.stream_id, dst);
                if stream.offset > 0 {
                    let _ = QuicVarInt::encode(stream.offset, dst);
                }
                let _ = QuicVarInt::encode(stream.data.len() as u64, dst);
                dst.extend_from_slice(&stream.data);
            }
            Frame::ResetStream {
                stream_id,
                error_code,
                final_size,
            } => {
                let _ = QuicVarInt::encode(0x04, dst);
                let _ = QuicVarInt::encode(*stream_id, dst);
                let _ = QuicVarInt::encode(*error_code, dst);
                let _ = QuicVarInt::encode(*final_size, dst);
            }
            Frame::StopSending {
                stream_id,
                error_code,
            } => {
                let _ = QuicVarInt::encode(0x05, dst);
                let _ = QuicVarInt::encode(*stream_id, dst);
                let _ = QuicVarInt::encode(*error_code, dst);
            }
            Frame::MaxData { max_data } => {
                let _ = QuicVarInt::encode(0x10, dst);
                let _ = QuicVarInt::encode(*max_data, dst);
            }
            Frame::MaxStreamData {
                stream_id,
                max_stream_data,
            } => {
                let _ = QuicVarInt::encode(0x11, dst);
                let _ = QuicVarInt::encode(*stream_id, dst);
                let _ = QuicVarInt::encode(*max_stream_data, dst);
            }
            Frame::MaxStreams {
                is_bidi,
                max_streams,
            } => {
                let frame_type = if *is_bidi { 0x12 } else { 0x13 };
                let _ = QuicVarInt::encode(frame_type, dst);
                let _ = QuicVarInt::encode(*max_streams, dst);
            }
            Frame::ConnectionClose {
                error_code,
                frame_type,
                reason_phrase,
            } => {
                if let Some(ft) = frame_type {
                    let _ = QuicVarInt::encode(0x1c, dst);
                    let _ = QuicVarInt::encode(*error_code, dst);
                    let _ = QuicVarInt::encode(*ft, dst);
                } else {
                    let _ = QuicVarInt::encode(0x1d, dst);
                    let _ = QuicVarInt::encode(*error_code, dst);
                }
                let bytes = reason_phrase.as_bytes();
                let _ = QuicVarInt::encode(bytes.len() as u64, dst);
                dst.extend_from_slice(bytes);
            }
            Frame::HandshakeDone => {
                let _ = QuicVarInt::encode(0x1e, dst);
            }
            _ => {
                let _ = QuicVarInt::encode(0x01, dst); // Ping fallback
            }
        }
    }

    pub fn decode(src: &mut Bytes) -> Result<Option<Frame>, &'static str> {
        if !src.has_remaining() {
            return Ok(None);
        }

        let frame_type = QuicVarInt::decode(src).map_err(|_| "invalid frame type varint")?;
        match frame_type {
            0x00 => {
                let mut count = 1;
                while src.has_remaining() && src.chunk()[0] == 0 {
                    src.advance(1);
                    count += 1;
                }
                Ok(Some(Frame::Padding { length: count }))
            }
            0x01 => Ok(Some(Frame::Ping)),
            0x02 | 0x03 => {
                let largest_ack = QuicVarInt::decode(src).map_err(|_| "ack largest")?;
                let ack_delay = QuicVarInt::decode(src).map_err(|_| "ack delay")?;
                let range_count = QuicVarInt::decode(src).map_err(|_| "ack range count")?;
                if range_count > 512 || src.remaining() < range_count as usize * 2 {
                    return Err("invalid ack range count");
                }
                let first_range = QuicVarInt::decode(src).map_err(|_| "ack first range")?;

                let mut ack_ranges = Vec::with_capacity(range_count.min(128) as usize);
                for _ in 0..range_count {
                    let gap = QuicVarInt::decode(src).map_err(|_| "ack gap")?;
                    let len = QuicVarInt::decode(src).map_err(|_| "ack range len")?;
                    ack_ranges.push(AckRange {
                        gap,
                        ack_range_length: len,
                    });
                }

                let ecn_counts = if frame_type == 0x03 {
                    let ect0 = QuicVarInt::decode(src).map_err(|_| "ecn ect0")?;
                    let ect1 = QuicVarInt::decode(src).map_err(|_| "ecn ect1")?;
                    let ce = QuicVarInt::decode(src).map_err(|_| "ecn ce")?;
                    Some(EcnCounts { ect0, ect1, ce })
                } else {
                    None
                };

                Ok(Some(Frame::Ack(AckFrame {
                    largest_acknowledged: largest_ack,
                    ack_delay,
                    first_ack_range: first_range,
                    ack_ranges,
                    ecn_counts,
                })))
            }
            0x04 => {
                let stream_id = QuicVarInt::decode(src).map_err(|_| "reset stream id")?;
                let error_code = QuicVarInt::decode(src).map_err(|_| "reset err code")?;
                let final_size = QuicVarInt::decode(src).map_err(|_| "reset final size")?;
                Ok(Some(Frame::ResetStream {
                    stream_id,
                    error_code,
                    final_size,
                }))
            }
            0x05 => {
                let stream_id = QuicVarInt::decode(src).map_err(|_| "stop sending stream id")?;
                let error_code = QuicVarInt::decode(src).map_err(|_| "stop sending err code")?;
                Ok(Some(Frame::StopSending {
                    stream_id,
                    error_code,
                }))
            }
            0x06 => {
                let offset = QuicVarInt::decode(src).map_err(|_| "crypto offset")?;
                let len = QuicVarInt::decode(src).map_err(|_| "crypto len")? as usize;
                if src.remaining() < len {
                    return Err("truncated crypto payload");
                }
                let data = src.split_to(len);
                Ok(Some(Frame::Crypto(CryptoFrame { offset, data })))
            }
            0x08..=0x0f => {
                let stream_id = QuicVarInt::decode(src).map_err(|_| "stream id")?;
                let offset = if (frame_type & 0x04) != 0 {
                    QuicVarInt::decode(src).map_err(|_| "stream offset")?
                } else {
                    0
                };
                let len = if (frame_type & 0x02) != 0 {
                    QuicVarInt::decode(src).map_err(|_| "stream len")? as usize
                } else {
                    src.remaining()
                };
                if src.remaining() < len {
                    return Err("truncated stream payload");
                }
                let fin = (frame_type & 0x01) != 0;
                let data = src.split_to(len);
                Ok(Some(Frame::Stream(StreamFrame {
                    stream_id,
                    offset,
                    fin,
                    data,
                })))
            }
            0x10 => {
                let max_data = QuicVarInt::decode(src).map_err(|_| "max data")?;
                Ok(Some(Frame::MaxData { max_data }))
            }
            0x11 => {
                let stream_id = QuicVarInt::decode(src).map_err(|_| "max stream data id")?;
                let max_stream_data = QuicVarInt::decode(src).map_err(|_| "max stream data")?;
                Ok(Some(Frame::MaxStreamData {
                    stream_id,
                    max_stream_data,
                }))
            }
            0x12 | 0x13 => {
                let max_streams = QuicVarInt::decode(src).map_err(|_| "max streams")?;
                Ok(Some(Frame::MaxStreams {
                    is_bidi: frame_type == 0x12,
                    max_streams,
                }))
            }
            0x1c | 0x1d => {
                let error_code = QuicVarInt::decode(src).map_err(|_| "close err code")?;
                let frame_type = if frame_type == 0x1c {
                    Some(QuicVarInt::decode(src).map_err(|_| "close frame type")?)
                } else {
                    None
                };
                let len = QuicVarInt::decode(src).map_err(|_| "reason len")? as usize;
                if src.remaining() < len {
                    return Err("truncated close reason");
                }
                let reason = String::from_utf8_lossy(&src.split_to(len)).to_string();
                Ok(Some(Frame::ConnectionClose {
                    error_code,
                    frame_type,
                    reason_phrase: reason,
                }))
            }
            0x1e => Ok(Some(Frame::HandshakeDone)),
            _ => Err("unsupported frame type"),
        }
    }
}
