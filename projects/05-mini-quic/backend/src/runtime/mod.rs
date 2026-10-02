//! Tokio Async UDP Runtime, Buffer Pooling, Parsing Budget & Metrics (05-069, 05-071, 05-072, 05-078, 05-079)

use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;

/// 05-071: Buffer Pooling with reuse ratio tracking
pub struct BufferPool {
    pool: Mutex<VecDeque<BytesMut>>,
    capacity_per_buffer: usize,
    allocations: AtomicUsize,
    reuses: AtomicUsize,
}

impl BufferPool {
    pub fn new(capacity_per_buffer: usize) -> Self {
        Self {
            pool: Mutex::new(VecDeque::new()),
            capacity_per_buffer,
            allocations: AtomicUsize::new(0),
            reuses: AtomicUsize::new(0),
        }
    }

    pub fn acquire(&self) -> BytesMut {
        let mut pool = self.pool.lock().unwrap();
        if let Some(mut buf) = pool.pop_front() {
            buf.clear();
            self.reuses.fetch_add(1, Ordering::Relaxed);
            buf
        } else {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            BytesMut::with_capacity(self.capacity_per_buffer)
        }
    }

    pub fn release(&self, buf: BytesMut) {
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < 1024 {
            pool.push_back(buf);
        }
    }

    pub fn reuse_ratio(&self) -> f64 {
        let a = self.allocations.load(Ordering::Relaxed) as f64;
        let r = self.reuses.load(Ordering::Relaxed) as f64;
        if a + r == 0.0 {
            0.0
        } else {
            r / (a + r)
        }
    }
}

/// 05-072: Packet Parsing Allocation Budget
pub struct PacketParseBudget {
    max_allocations_per_parse: usize,
}

impl PacketParseBudget {
    pub fn new(max_allocs: usize) -> Self {
        Self {
            max_allocations_per_parse: max_allocs,
        }
    }

    pub fn parse_zero_alloc<'a>(&self, raw: &'a [u8]) -> Result<(u8, u64, &'a [u8]), &'static str> {
        if raw.len() < 9 {
            return Err("packet too short");
        }
        let flags = raw[0];
        let pn = u64::from_be_bytes(raw[1..9].try_into().unwrap());
        let payload = &raw[9..];
        Ok((flags, pn, payload))
    }

    pub fn max_allocations(&self) -> usize {
        self.max_allocations_per_parse
    }
}

/// 05-069: Tokio UDP Socket Runtime
pub struct TokioUdpRuntime {
    pub socket: Arc<UdpSocket>,
}

impl TokioUdpRuntime {
    pub async fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> std::io::Result<usize> {
        self.socket.send_to(buf, target).await
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(buf).await
    }
}

/// 05-078: High-Throughput Benchmark Harness Report
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThroughputBenchmarkReport {
    pub git_sha: String,
    pub rustc_version: String,
    pub cpu_cores: usize,
    pub os: String,
    pub total_bytes_transferred: u64,
    pub duration_seconds: f64,
    pub throughput_mbps: f64,
}

/// 05-079: Latency Percentiles Measurement
pub struct LatencyMeasurement {
    samples: Mutex<Vec<u64>>,
}

impl Default for LatencyMeasurement {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyMeasurement {
    pub fn new() -> Self {
        Self {
            samples: Mutex::new(Vec::new()),
        }
    }

    pub fn record(&self, latency_us: u64) {
        self.samples.lock().unwrap().push(latency_us);
    }

    pub fn percentiles(&self) -> (u64, u64, u64) {
        let mut samples = self.samples.lock().unwrap().clone();
        if samples.is_empty() {
            return (0, 0, 0);
        }
        samples.sort_unstable();
        let len = samples.len();
        let p50 = samples[(len as f64 * 0.50) as usize];
        let p95 = samples[((len as f64 * 0.95) as usize).min(len - 1)];
        let p99 = samples[((len as f64 * 0.99) as usize).min(len - 1)];
        (p50, p95, p99)
    }
}
