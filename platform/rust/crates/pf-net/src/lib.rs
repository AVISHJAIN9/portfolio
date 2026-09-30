use pf_errors::{PlatformError, PlatformResult};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

/// High-throughput UDP datagram framing wrapper.
pub struct UdpFramedEndpoint {
    socket: UdpSocket,
}

impl UdpFramedEndpoint {
    pub async fn bind(addr: &str) -> PlatformResult<Self> {
        let socket = UdpSocket::bind(addr)
            .await
            .map_err(|e| PlatformError::Io(e.to_string()))?;
        Ok(Self { socket })
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> PlatformResult<usize> {
        self.socket
            .send_to(buf, target)
            .await
            .map_err(|e| PlatformError::Io(e.to_string()))
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> PlatformResult<(usize, SocketAddr)> {
        self.socket
            .recv_from(buf)
            .await
            .map_err(|e| PlatformError::Io(e.to_string()))
    }

    pub fn local_addr(&self) -> PlatformResult<SocketAddr> {
        self.socket
            .local_addr()
            .map_err(|e| PlatformError::Io(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_udp_endpoint() {
        let ep1 = UdpFramedEndpoint::bind("127.0.0.1:0").await.unwrap();
        let ep2 = UdpFramedEndpoint::bind("127.0.0.1:0").await.unwrap();

        let addr2 = ep2.local_addr().unwrap();
        ep1.send_to(b"hello_netcode", addr2).await.unwrap();

        let mut buf = [0u8; 64];
        let (n, from) = ep2.recv_from(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello_netcode");
        assert_eq!(from, ep1.local_addr().unwrap());
    }
}
