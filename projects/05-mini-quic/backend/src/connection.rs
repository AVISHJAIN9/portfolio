//! QUIC Connection State Machine (RFC 9000 Section 5)

use bytes::Bytes;
use std::time::Duration;
use crate::crypto::CryptoContext;
use crate::recovery::LossRecovery;
use crate::stream::StreamManager;
use crate::wire::codec::PacketCodec;
use crate::wire::frame::{AckFrame, Frame, StreamFrame};
use crate::wire::packet::{ConnectionId, Header, Packet, PacketType, QUIC_VERSION_1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Initial,
    Handshake,
    OneRtt,
    Draining,
    Closed,
}

pub struct QuicConnection {
    pub state: ConnectionState,
    pub dcid: ConnectionId,
    pub scid: ConnectionId,
    pub crypto: CryptoContext,
    pub recovery: LossRecovery,
    pub streams: StreamManager,
    pub next_packet_number: u64,
    pub is_server: bool,
}

impl QuicConnection {
    pub fn new_client(server_cid: ConnectionId) -> Self {
        let scid = ConnectionId::random(8);
        let crypto = CryptoContext::new_initial(&server_cid.0);
        Self {
            state: ConnectionState::Initial,
            dcid: server_cid,
            scid,
            crypto,
            recovery: LossRecovery::new(),
            streams: StreamManager::new(1024 * 1024),
            next_packet_number: 0,
            is_server: false,
        }
    }

    pub fn new_server(client_cid: ConnectionId, server_cid: ConnectionId) -> Self {
        let crypto = CryptoContext::new_initial(&server_cid.0);
        Self {
            state: ConnectionState::Initial,
            dcid: client_cid,
            scid: server_cid,
            crypto,
            recovery: LossRecovery::new(),
            streams: StreamManager::new(1024 * 1024),
            next_packet_number: 0,
            is_server: true,
        }
    }

    pub fn handle_packet(&mut self, mut raw_bytes: Bytes) -> Result<Option<Packet>, &'static str> {
        let packet = PacketCodec::decode(&mut raw_bytes).map_err(|_| "Failed to decode QUIC packet")?;

        for frame in &packet.frames {
            match frame {
                Frame::Ping => {}
                Frame::Ack(ack) => {
                    self.recovery.on_ack_received(ack.largest_acknowledged, Duration::from_millis(ack.ack_delay));
                }
                Frame::Crypto(_) => {
                    if self.state == ConnectionState::Initial {
                        self.state = ConnectionState::Handshake;
                    } else if self.state == ConnectionState::Handshake {
                        self.state = ConnectionState::OneRtt;
                    }
                }
                Frame::Stream(stream) => {
                    let s = self.streams.get_or_create(stream.stream_id, false, 256 * 1024);
                    s.receive_data(stream.offset, stream.data.clone(), stream.fin);
                }
                Frame::ConnectionClose { .. } => {
                    self.state = ConnectionState::Closed;
                }
                _ => {}
            }
        }

        // Generate response ACK packet
        let resp_packet = if !packet.frames.is_empty() {
            let pn = self.next_packet_number;
            self.next_packet_number += 1;

            let ack_frame = Frame::Ack(AckFrame {
                largest_acknowledged: packet.header.packet_number(),
                ack_delay: 1,
                first_ack_range: 0,
                ack_ranges: vec![],
                ecn_counts: None,
            });

            let header = if self.state == ConnectionState::OneRtt {
                Header::Short {
                    spin_bit: false,
                    key_phase: false,
                    dcid: self.dcid.clone(),
                    packet_number: pn,
                    packet_number_len: 2,
                }
            } else {
                Header::Long {
                    packet_type: if self.state == ConnectionState::Initial {
                        PacketType::Initial
                    } else {
                        PacketType::Handshake
                    },
                    version: QUIC_VERSION_1,
                    dcid: self.dcid.clone(),
                    scid: self.scid.clone(),
                    token: Bytes::new(),
                    packet_number: pn,
                    packet_number_len: 2,
                }
            };

            Some(Packet {
                header,
                frames: vec![ack_frame],
            })
        } else {
            None
        };

        Ok(resp_packet)
    }

    pub fn send_stream_data(&mut self, stream_id: u64, data: &[u8], fin: bool) -> Result<Packet, &'static str> {
        if self.state != ConnectionState::OneRtt {
            return Err("Connection not in 1-RTT state");
        }

        let s = self.streams.get_or_create(stream_id, true, 256 * 1024);
        let offset = s.send_offset;
        s.write_data(data, fin)?;

        let pn = self.next_packet_number;
        self.next_packet_number += 1;

        let frame = Frame::Stream(StreamFrame {
            stream_id,
            offset,
            fin,
            data: Bytes::copy_from_slice(data),
        });

        let packet = Packet {
            header: Header::Short {
                spin_bit: false,
                key_phase: false,
                dcid: self.dcid.clone(),
                packet_number: pn,
                packet_number_len: 2,
            },
            frames: vec![frame],
        };

        self.recovery.on_packet_sent(pn, data.len() as u64 + 32, true);
        Ok(packet)
    }
}
