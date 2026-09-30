//! QUIC Streams and Flow Control (RFC 9000 Section 2.1 & Section 4)

use bytes::{Bytes, BytesMut};
use std::collections::{BTreeMap, HashMap};

pub struct Stream {
    pub stream_id: u64,
    pub is_bidi: bool,
    pub is_local: bool,
    pub send_offset: u64,
    pub recv_offset: u64,
    pub send_max_stream_data: u64,
    pub recv_max_stream_data: u64,
    pub send_buffer: BytesMut,
    pub recv_fragments: BTreeMap<u64, Bytes>,
    pub is_fin_received: bool,
    pub is_fin_sent: bool,
}

impl Stream {
    pub fn new(stream_id: u64, is_local: bool, max_stream_data: u64) -> Self {
        let is_bidi = (stream_id & 0x02) == 0;
        Self {
            stream_id,
            is_bidi,
            is_local,
            send_offset: 0,
            recv_offset: 0,
            send_max_stream_data: max_stream_data,
            recv_max_stream_data: max_stream_data,
            send_buffer: BytesMut::new(),
            recv_fragments: BTreeMap::new(),
            is_fin_received: false,
            is_fin_sent: false,
        }
    }

    pub fn write_data(&mut self, data: &[u8], fin: bool) -> Result<u64, &'static str> {
        let len = data.len() as u64;
        if self.send_offset + len > self.send_max_stream_data {
            return Err("Stream flow control limit exceeded");
        }
        self.send_buffer.extend_from_slice(data);
        self.send_offset += len;
        if fin {
            self.is_fin_sent = true;
        }
        Ok(len)
    }

    pub fn receive_data(&mut self, offset: u64, data: Bytes, fin: bool) {
        if fin {
            self.is_fin_received = true;
        }
        self.recv_fragments.insert(offset, data);
    }

    pub fn read_contiguous(&mut self) -> Option<Bytes> {
        if let Some(chunk) = self.recv_fragments.remove(&self.recv_offset) {
            self.recv_offset += chunk.len() as u64;
            Some(chunk)
        } else {
            None
        }
    }
}

pub struct StreamManager {
    pub streams: HashMap<u64, Stream>,
    pub max_data: u64,
    pub data_sent: u64,
    pub data_received: u64,
}

impl StreamManager {
    pub fn new(initial_max_data: u64) -> Self {
        Self {
            streams: HashMap::new(),
            max_data: initial_max_data,
            data_sent: 0,
            data_received: 0,
        }
    }

    pub fn get_or_create(&mut self, stream_id: u64, is_local: bool, max_stream_data: u64) -> &mut Stream {
        self.streams.entry(stream_id).or_insert_with(|| Stream::new(stream_id, is_local, max_stream_data))
    }
}
