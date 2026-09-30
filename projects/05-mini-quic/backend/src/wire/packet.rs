//! QUIC Packet Header & Types (RFC 9000 Section 17)

use bytes::{BufMut, Bytes, BytesMut};
use pf_bytes::QuicVarInt;
use crate::wire::frame::Frame;

pub const QUIC_VERSION_1: u32 = 0x00000001;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketType {
    Initial,
    ZeroRtt,
    Handshake,
    Retry,
    OneRtt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionId(pub Vec<u8>);

impl ConnectionId {
    pub fn new(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }

    pub fn random(len: usize) -> Self {
        let mut buf = vec![0u8; len];
        rand::Rng::fill(&mut rand::thread_rng(), &mut buf[..]);
        Self(buf)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Header {
    Long {
        packet_type: PacketType,
        version: u32,
        dcid: ConnectionId,
        scid: ConnectionId,
        token: Bytes,
        packet_number: u64,
        packet_number_len: usize,
    },
    Short {
        spin_bit: bool,
        key_phase: bool,
        dcid: ConnectionId,
        packet_number: u64,
        packet_number_len: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub header: Header,
    pub frames: Vec<Frame>,
}

impl Header {
    pub fn packet_number(&self) -> u64 {
        match self {
            Header::Long { packet_number, .. } => *packet_number,
            Header::Short { packet_number, .. } => *packet_number,
        }
    }

    pub fn dcid(&self) -> &ConnectionId {
        match self {
            Header::Long { dcid, .. } => dcid,
            Header::Short { dcid, .. } => dcid,
        }
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        match self {
            Header::Long {
                packet_type,
                version,
                dcid,
                scid,
                token,
                packet_number,
                packet_number_len,
            } => {
                let type_bits = match packet_type {
                    PacketType::Initial => 0x00,
                    PacketType::ZeroRtt => 0x01,
                    PacketType::Handshake => 0x02,
                    PacketType::Retry => 0x03,
                    PacketType::OneRtt => 0x00,
                };
                let pn_len_bits = (*packet_number_len - 1) as u8 & 0x03;
                let first_byte = 0x80 | 0x40 | (type_bits << 4) | pn_len_bits;
                dst.put_u8(first_byte);
                dst.put_u32(*version);

                // DCID
                dst.put_u8(dcid.0.len() as u8);
                dst.extend_from_slice(&dcid.0);

                // SCID
                dst.put_u8(scid.0.len() as u8);
                dst.extend_from_slice(&scid.0);

                // Token
                if *packet_type == PacketType::Initial {
                    let _ = QuicVarInt::encode(token.len() as u64, dst);
                    dst.extend_from_slice(token);
                }

                // Packet Number (encoded directly)
                encode_packet_number(*packet_number, *packet_number_len, dst);
            }
            Header::Short {
                spin_bit,
                key_phase,
                dcid,
                packet_number,
                packet_number_len,
            } => {
                let pn_len_bits = (*packet_number_len - 1) as u8 & 0x03;
                let mut first_byte = 0x40 | pn_len_bits;
                if *spin_bit {
                    first_byte |= 0x20;
                }
                if *key_phase {
                    first_byte |= 0x04;
                }
                dst.put_u8(first_byte);
                dst.extend_from_slice(&dcid.0);
                encode_packet_number(*packet_number, *packet_number_len, dst);
            }
        }
    }
}

fn encode_packet_number(pn: u64, len: usize, dst: &mut BytesMut) {
    match len {
        1 => dst.put_u8(pn as u8),
        2 => dst.put_u16(pn as u16),
        3 => {
            dst.put_u8((pn >> 16) as u8);
            dst.put_u16(pn as u16);
        }
        4 => dst.put_u32(pn as u32),
        _ => dst.put_u32(pn as u32),
    }
}
