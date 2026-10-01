//! QUIC Packet & Frame Codec (RFC 9000 Section 17)

use bytes::{Buf, Bytes, BytesMut};
use pf_bytes::QuicVarInt;
use crate::wire::frame::Frame;
use crate::wire::packet::{ConnectionId, Header, Packet, PacketType, QUIC_VERSION_1};

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("Buffer underflow: {0}")]
    Underflow(&'static str),
    #[error("Invalid QUIC version: {0}")]
    InvalidVersion(u32),
    #[error("Invalid frame payload: {0}")]
    InvalidFrame(&'static str),
    #[error("Malformed packet header")]
    MalformedHeader,
}

pub struct PacketCodec;

impl PacketCodec {
    pub fn encode(packet: &Packet, dst: &mut BytesMut) {
        packet.header.encode(dst);

        let mut payload_buf = BytesMut::new();
        for frame in &packet.frames {
            frame.encode(&mut payload_buf);
        }

        // Payload length varint + payload
        let _ = QuicVarInt::encode(payload_buf.len() as u64, dst);
        dst.extend_from_slice(&payload_buf);
    }

    pub fn decode(src: &mut Bytes) -> Result<Packet, CodecError> {
        if src.remaining() < 5 {
            return Err(CodecError::Underflow("Packet too short"));
        }

        let first_byte = src.get_u8();
        let is_long_header = (first_byte & 0x80) != 0;

        let header = if is_long_header {
            if src.remaining() < 4 {
                return Err(CodecError::Underflow("Version truncated"));
            }
            let version = src.get_u32();
            if version != QUIC_VERSION_1 && version != 0 {
                return Err(CodecError::InvalidVersion(version));
            }

            if src.remaining() < 1 {
                return Err(CodecError::Underflow("DCID length missing"));
            }
            let dcid_len = src.get_u8() as usize;
            if dcid_len > 20 || src.remaining() < dcid_len {
                return Err(CodecError::MalformedHeader);
            }
            let dcid = ConnectionId(src.split_to(dcid_len).to_vec());

            if src.remaining() < 1 {
                return Err(CodecError::Underflow("SCID length missing"));
            }
            let scid_len = src.get_u8() as usize;
            if scid_len > 20 || src.remaining() < scid_len {
                return Err(CodecError::MalformedHeader);
            }
            let scid = ConnectionId(src.split_to(scid_len).to_vec());

            let type_bits = (first_byte >> 4) & 0x03;
            let packet_type = match type_bits {
                0x00 => PacketType::Initial,
                0x01 => PacketType::ZeroRtt,
                0x02 => PacketType::Handshake,
                0x03 => PacketType::Retry,
                _ => PacketType::Initial,
            };

            let token = if packet_type == PacketType::Initial {
                let token_len = QuicVarInt::decode(src).map_err(|_| CodecError::MalformedHeader)? as usize;
                if src.remaining() < token_len {
                    return Err(CodecError::Underflow("Token truncated"));
                }
                src.split_to(token_len)
            } else {
                Bytes::new()
            };

            let pn_len = ((first_byte & 0x03) + 1) as usize;
            let pn = decode_packet_number(src, pn_len)?;

            Header::Long {
                packet_type,
                version,
                dcid,
                scid,
                token,
                packet_number: pn,
                packet_number_len: pn_len,
            }
        } else {
            let spin_bit = (first_byte & 0x20) != 0;
            let key_phase = (first_byte & 0x04) != 0;
            if src.remaining() < 8 {
                return Err(CodecError::Underflow("Short header DCID truncated"));
            }
            let dcid = ConnectionId(src.split_to(8).to_vec()); // standard 8-byte short header DCID
            let pn_len = ((first_byte & 0x03) + 1) as usize;
            let pn = decode_packet_number(src, pn_len)?;

            Header::Short {
                spin_bit,
                key_phase,
                dcid,
                packet_number: pn,
                packet_number_len: pn_len,
            }
        };

        // Decode payload length
        let payload_len = QuicVarInt::decode(src).map_err(|_| CodecError::MalformedHeader)? as usize;
        if src.remaining() < payload_len {
            return Err(CodecError::Underflow("Payload truncated"));
        }

        let mut payload = src.split_to(payload_len);
        let mut frames = Vec::new();

        while payload.has_remaining() {
            if let Some(frame) = Frame::decode(&mut payload).map_err(CodecError::InvalidFrame)? {
                frames.push(frame);
            }
        }

        Ok(Packet { header, frames })
    }
}

fn decode_packet_number(src: &mut Bytes, len: usize) -> Result<u64, CodecError> {
    if src.remaining() < len {
        return Err(CodecError::Underflow("Packet number truncated"));
    }
    match len {
        1 => Ok(src.get_u8() as u64),
        2 => Ok(src.get_u16() as u64),
        3 => {
            let b0 = src.get_u8() as u64;
            let b12 = src.get_u16() as u64;
            Ok((b0 << 16) | b12)
        }
        4 => Ok(src.get_u32() as u64),
        _ => Ok(src.get_u32() as u64),
    }
}
