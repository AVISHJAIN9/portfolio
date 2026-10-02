//! QUIC Streams and Flow Control (RFC 9000 Section 2.1 & Section 4)

use bytes::{Bytes, BytesMut};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    ClientBidi = 0x00,
    ServerBidi = 0x01,
    ClientUnidi = 0x02,
    ServerUnidi = 0x03,
}

impl StreamType {
    pub fn from_stream_id(stream_id: u64) -> Self {
        match stream_id & 0x03 {
            0x00 => StreamType::ClientBidi,
            0x01 => StreamType::ServerBidi,
            0x02 => StreamType::ClientUnidi,
            0x03 => StreamType::ServerUnidi,
            _ => unreachable!(),
        }
    }

    pub fn is_bidi(self) -> bool {
        matches!(self, StreamType::ClientBidi | StreamType::ServerBidi)
    }

    pub fn is_client_initiated(self) -> bool {
        matches!(self, StreamType::ClientBidi | StreamType::ClientUnidi)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Open,
    SendClosed,
    RecvClosed,
    Closed,
    ResetSent(u64),
    ResetReceived(u64),
}

pub struct Stream {
    pub stream_id: u64,
    pub stream_type: StreamType,
    pub is_local: bool,
    pub state: StreamState,
    pub send_offset: u64,
    pub recv_offset: u64,
    pub send_max_stream_data: u64,
    pub recv_max_stream_data: u64,
    pub send_buffer: BytesMut,
    pub recv_fragments: BTreeMap<u64, Bytes>,
    pub is_fin_received: bool,
    pub is_fin_sent: bool,
    pub stop_sending_received: bool,
}

impl Stream {
    pub fn new(stream_id: u64, is_local: bool, max_stream_data: u64) -> Self {
        let stream_type = StreamType::from_stream_id(stream_id);
        Self {
            stream_id,
            stream_type,
            is_local,
            state: StreamState::Open,
            send_offset: 0,
            recv_offset: 0,
            send_max_stream_data: max_stream_data,
            recv_max_stream_data: max_stream_data,
            send_buffer: BytesMut::new(),
            recv_fragments: BTreeMap::new(),
            is_fin_received: false,
            is_fin_sent: false,
            stop_sending_received: false,
        }
    }

    pub fn write_data(&mut self, data: &[u8], fin: bool) -> Result<u64, &'static str> {
        if matches!(
            self.state,
            StreamState::SendClosed | StreamState::Closed | StreamState::ResetSent(_)
        ) {
            return Err("Stream send side is closed or reset");
        }
        let len = data.len() as u64;
        if self.send_offset + len > self.send_max_stream_data {
            return Err("Stream flow control limit exceeded (MAX_STREAM_DATA)");
        }
        self.send_buffer.extend_from_slice(data);
        self.send_offset += len;
        if fin {
            self.is_fin_sent = true;
            if self.is_fin_received {
                self.state = StreamState::Closed;
            } else {
                self.state = StreamState::SendClosed;
            }
        }
        Ok(len)
    }

    pub fn receive_data(
        &mut self,
        offset: u64,
        data: Bytes,
        fin: bool,
    ) -> Result<(), &'static str> {
        let len = data.len() as u64;
        if offset + len > self.recv_max_stream_data {
            return Err("Received data exceeds stream flow control limit");
        }
        if fin {
            self.is_fin_received = true;
            if self.is_fin_sent {
                self.state = StreamState::Closed;
            } else {
                self.state = StreamState::RecvClosed;
            }
        }
        self.recv_fragments.insert(offset, data);
        Ok(())
    }

    pub fn read_contiguous(&mut self) -> Option<Bytes> {
        if let Some(chunk) = self.recv_fragments.remove(&self.recv_offset) {
            self.recv_offset += chunk.len() as u64;
            Some(chunk)
        } else {
            None
        }
    }

    pub fn reset(&mut self, error_code: u64) {
        self.state = StreamState::ResetSent(error_code);
        self.send_buffer.clear();
    }

    pub fn on_reset_received(&mut self, error_code: u64, final_size: u64) {
        self.state = StreamState::ResetReceived(error_code);
        self.recv_fragments.clear();
        self.recv_offset = final_size;
    }
}

pub struct StreamManager {
    pub streams: HashMap<u64, Stream>,
    pub max_data: u64,
    pub max_stream_data_bidi_local: u64,
    pub max_stream_data_bidi_remote: u64,
    pub max_stream_data_uni: u64,
    pub max_streams_bidi: u64,
    pub max_streams_uni: u64,
    pub next_client_bidi_id: u64,
    pub next_client_uni_id: u64,
    pub next_server_bidi_id: u64,
    pub next_server_uni_id: u64,
    pub data_sent: u64,
    pub data_received: u64,
    pub is_server: bool,
}

impl StreamManager {
    pub fn new(initial_max_data: u64, initial_max_stream_data: u64, is_server: bool) -> Self {
        Self {
            streams: HashMap::new(),
            max_data: initial_max_data,
            max_stream_data_bidi_local: initial_max_stream_data,
            max_stream_data_bidi_remote: initial_max_stream_data,
            max_stream_data_uni: initial_max_stream_data,
            max_streams_bidi: 100,
            max_streams_uni: 100,
            next_client_bidi_id: 0,
            next_client_uni_id: 2,
            next_server_bidi_id: 1,
            next_server_uni_id: 3,
            data_sent: 0,
            data_received: 0,
            is_server,
        }
    }

    pub fn get_or_create(
        &mut self,
        stream_id: u64,
        is_local: bool,
        max_stream_data: u64,
    ) -> &mut Stream {
        self.streams
            .entry(stream_id)
            .or_insert_with(|| Stream::new(stream_id, is_local, max_stream_data))
    }

    /// Allocate a new stream ID according to initiator and bidirectionality
    pub fn allocate_stream_id(&mut self, is_bidi: bool) -> Result<u64, &'static str> {
        let (id, limit) = if self.is_server {
            if is_bidi {
                let id = self.next_server_bidi_id;
                let count = (id >> 2) + 1;
                if count > self.max_streams_bidi {
                    return Err("MAX_STREAMS limit reached for bidirectional streams");
                }
                self.next_server_bidi_id += 4;
                (id, self.max_stream_data_bidi_local)
            } else {
                let id = self.next_server_uni_id;
                let count = (id >> 2) + 1;
                if count > self.max_streams_uni {
                    return Err("MAX_STREAMS limit reached for unidirectional streams");
                }
                self.next_server_uni_id += 4;
                (id, self.max_stream_data_uni)
            }
        } else {
            if is_bidi {
                let id = self.next_client_bidi_id;
                let count = (id >> 2) + 1;
                if count > self.max_streams_bidi {
                    return Err("MAX_STREAMS limit reached for bidirectional streams");
                }
                self.next_client_bidi_id += 4;
                (id, self.max_stream_data_bidi_local)
            } else {
                let id = self.next_client_uni_id;
                let count = (id >> 2) + 1;
                if count > self.max_streams_uni {
                    return Err("MAX_STREAMS limit reached for unidirectional streams");
                }
                self.next_client_uni_id += 4;
                (id, self.max_stream_data_uni)
            }
        };

        self.streams.insert(id, Stream::new(id, true, limit));
        Ok(id)
    }

    pub fn send_stream_data(
        &mut self,
        stream_id: u64,
        data: &[u8],
        fin: bool,
    ) -> Result<u64, &'static str> {
        let len = data.len() as u64;
        if self.data_sent + len > self.max_data {
            return Err("Connection flow control limit exceeded (MAX_DATA)");
        }
        let stream = self.streams.get_mut(&stream_id).ok_or("Stream not found")?;
        let written = stream.write_data(data, fin)?;
        self.data_sent += written;
        Ok(written)
    }

    pub fn receive_stream_data(
        &mut self,
        stream_id: u64,
        offset: u64,
        data: Bytes,
        fin: bool,
    ) -> Result<(), &'static str> {
        let len = data.len() as u64;
        if self.data_received + len > self.max_data {
            return Err("Connection flow control limit exceeded on receive (MAX_DATA)");
        }
        let max_stream_data = self.max_stream_data_bidi_remote;
        let stream = self
            .streams
            .entry(stream_id)
            .or_insert_with(|| Stream::new(stream_id, false, max_stream_data));
        stream.receive_data(offset, data, fin)?;
        self.data_received += len;
        Ok(())
    }

    pub fn update_max_data(&mut self, max_data: u64) {
        if max_data > self.max_data {
            self.max_data = max_data;
        }
    }

    pub fn update_max_stream_data(
        &mut self,
        stream_id: u64,
        max_stream_data: u64,
    ) -> Result<(), &'static str> {
        let stream = self.streams.get_mut(&stream_id).ok_or("Stream not found")?;
        if max_stream_data > stream.send_max_stream_data {
            stream.send_max_stream_data = max_stream_data;
        }
        Ok(())
    }

    pub fn update_max_streams_bidi(&mut self, max_streams: u64) {
        if max_streams > self.max_streams_bidi {
            self.max_streams_bidi = max_streams;
        }
    }

    pub fn update_max_streams_uni(&mut self, max_streams: u64) {
        if max_streams > self.max_streams_uni {
            self.max_streams_uni = max_streams;
        }
    }
}
