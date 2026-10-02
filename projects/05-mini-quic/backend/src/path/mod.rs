//! Path State Management, Validation, Migration, and NAT Rebinding (05-058..05-063)

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathState {
    New,
    Validating,
    Validated,
    Failed,
}

#[derive(Debug, Clone)]
pub struct QuicPath {
    pub local_addr: SocketAddr,
    pub peer_addr: SocketAddr,
    pub state: PathState,
    pub challenge_data: Option<[u8; 8]>,
    pub challenge_sent_at: Option<Instant>,
    pub pto_count: u32,
    pub bytes_sent_unvalidated: usize,
}

impl QuicPath {
    pub fn new(local_addr: SocketAddr, peer_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            peer_addr,
            state: PathState::New,
            challenge_data: None,
            challenge_sent_at: None,
            pto_count: 0,
            bytes_sent_unvalidated: 0,
        }
    }
}

pub struct PathManager {
    paths: HashMap<SocketAddr, QuicPath>,
    active_path: Option<SocketAddr>,
    anti_amplification_limit: usize,
}

impl PathManager {
    pub fn new(initial_local: SocketAddr, initial_peer: SocketAddr) -> Self {
        let mut paths = HashMap::new();
        let mut path = QuicPath::new(initial_local, initial_peer);
        path.state = PathState::Validated;
        paths.insert(initial_peer, path);
        Self {
            paths,
            active_path: Some(initial_peer),
            anti_amplification_limit: 3 * 1200, // 3x amplification limit
        }
    }

    pub fn anti_amplification_limit(&self) -> usize {
        self.anti_amplification_limit
    }

    /// 05-058: Path state management
    pub fn transition_state(
        &mut self,
        peer_addr: SocketAddr,
        new_state: PathState,
    ) -> Result<(), &'static str> {
        let path = self.paths.get_mut(&peer_addr).ok_or("path not found")?;
        match (path.state, new_state) {
            (PathState::New, PathState::Validating) => path.state = new_state,
            (PathState::Validating, PathState::Validated) => path.state = new_state,
            (PathState::Validating, PathState::Failed) => path.state = new_state,
            (PathState::Validated, PathState::Validating) => path.state = new_state,
            (PathState::Validated, PathState::Failed) => path.state = new_state,
            _ => return Err("illegal path state transition"),
        }
        Ok(())
    }

    /// 05-059: Path validation with 8-byte PATH_CHALLENGE / PATH_RESPONSE
    pub fn start_path_validation(
        &mut self,
        peer_addr: SocketAddr,
        challenge: [u8; 8],
        now: Instant,
    ) -> Result<(), &'static str> {
        self.transition_state(peer_addr, PathState::Validating)?;
        let path = self.paths.get_mut(&peer_addr).unwrap();
        path.challenge_data = Some(challenge);
        path.challenge_sent_at = Some(now);
        Ok(())
    }

    pub fn handle_path_response(
        &mut self,
        peer_addr: SocketAddr,
        response_data: [u8; 8],
    ) -> Result<bool, &'static str> {
        let path = self.paths.get_mut(&peer_addr).ok_or("path not found")?;
        if let Some(expected) = path.challenge_data {
            if response_data == expected {
                path.state = PathState::Validated;
                path.challenge_data = None;
                path.pto_count = 0;
                Ok(true)
            } else {
                Ok(false) // Mismatch
            }
        } else {
            Ok(false)
        }
    }

    pub fn check_validation_timeout(
        &mut self,
        peer_addr: SocketAddr,
        now: Instant,
        timeout: Duration,
    ) -> bool {
        if let Some(path) = self.paths.get_mut(&peer_addr) {
            if path.state == PathState::Validating {
                if let Some(sent_at) = path.challenge_sent_at {
                    if now.duration_since(sent_at) >= timeout {
                        path.state = PathState::Failed;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 05-060: Migration trigger logic
    pub fn try_migrate(&mut self, new_peer_addr: SocketAddr) -> Result<(), &'static str> {
        let path = self.paths.get(&new_peer_addr).ok_or("path not found")?;
        if path.state != PathState::Validated {
            return Err("migration refused: path is not validated");
        }
        self.active_path = Some(new_peer_addr);
        Ok(())
    }

    /// 05-061: Active path selection
    pub fn select_active_path(&mut self) -> Option<SocketAddr> {
        let mut validated_paths: Vec<_> = self
            .paths
            .iter()
            .filter(|(_, p)| p.state == PathState::Validated)
            .collect();
        validated_paths.sort_by_key(|(addr, p)| (p.pto_count, addr.to_string()));
        let selected = validated_paths.first().map(|(&addr, _)| addr);
        if let Some(addr) = selected {
            self.active_path = Some(addr);
        }
        selected
    }

    /// 05-062: Path failure detection & fallback
    pub fn record_pto(&mut self, max_consecutive_ptos: u32) -> Option<SocketAddr> {
        if let Some(active) = self.active_path {
            if let Some(path) = self.paths.get_mut(&active) {
                path.pto_count += 1;
                if path.pto_count >= max_consecutive_ptos {
                    path.state = PathState::Failed;
                    return self.select_active_path();
                }
            }
        }
        None
    }

    /// 05-063: NAT rebinding support
    pub fn handle_nat_rebinding(
        &mut self,
        local_addr: SocketAddr,
        new_peer_addr: SocketAddr,
    ) -> bool {
        if self.paths.contains_key(&new_peer_addr) {
            return false;
        }
        let path = QuicPath::new(local_addr, new_peer_addr);
        self.paths.insert(new_peer_addr, path);
        true
    }

    pub fn get_path(&self, peer_addr: &SocketAddr) -> Option<&QuicPath> {
        self.paths.get(peer_addr)
    }

    pub fn active_path(&self) -> Option<SocketAddr> {
        self.active_path
    }
}
