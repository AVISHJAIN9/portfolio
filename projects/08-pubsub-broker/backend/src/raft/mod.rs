//! Replication & Raft-Lite (Features 08-053..08-065)

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaRole {
    Leader,
    Follower,
    Candidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationState {
    pub node_id: u64,
    pub role: ReplicaRole,
    pub current_term: u64,
    pub voted_for: Option<u64>,
    pub leader_id: Option<u64>,
    pub match_offsets: HashMap<u64, u64>, // node_id -> LEO
    pub high_watermark: u64,
    pub log_end_offset: u64,
}

/// 08-053..08-065: Raft-Lite Replica State Machine
pub struct RaftReplica {
    state: RwLock<ReplicationState>,
    quorum_size: usize,
}

impl RaftReplica {
    pub fn new(node_id: u64, peers: &[u64]) -> Self {
        let total = peers.len() + 1;
        let quorum_size = (total / 2) + 1;
        let mut match_offsets = HashMap::new();
        for &p in peers {
            match_offsets.insert(p, 0);
        }
        match_offsets.insert(node_id, 0);

        Self {
            state: RwLock::new(ReplicationState {
                node_id,
                role: ReplicaRole::Follower,
                current_term: 0,
                voted_for: None,
                leader_id: None,
                match_offsets,
                high_watermark: 0,
                log_end_offset: 0,
            }),
            quorum_size,
        }
    }

    /// 08-054: Initiate leader election
    pub fn start_election(&self) -> bool {
        let mut s = self.state.write();
        s.role = ReplicaRole::Candidate;
        s.current_term += 1;
        s.voted_for = Some(s.node_id);
        s.leader_id = None;

        // In single-node or quorum 1, elect immediately
        if self.quorum_size <= 1 {
            s.role = ReplicaRole::Leader;
            s.leader_id = Some(s.node_id);
            true
        } else {
            false
        }
    }

    /// 08-054: Complete election with quorum votes
    pub fn become_leader(&self) {
        let mut s = self.state.write();
        s.role = ReplicaRole::Leader;
        s.leader_id = Some(s.node_id);
    }

    /// 08-059: Receive heartbeat from leader
    pub fn handle_heartbeat(&self, leader_id: u64, term: u64, leader_hw: u64) -> bool {
        let mut s = self.state.write();
        if term >= s.current_term {
            s.current_term = term;
            s.role = ReplicaRole::Follower;
            s.leader_id = Some(leader_id);
            if leader_hw > s.high_watermark {
                s.high_watermark = leader_hw.min(s.log_end_offset);
            }
            true
        } else {
            false
        }
    }

    /// 08-056..08-057: Update follower offset and recompute High Watermark via Quorum
    pub fn update_follower_offset(&self, follower_id: u64, leo: u64) -> u64 {
        let mut s = self.state.write();
        s.match_offsets.insert(follower_id, leo);

        // Compute quorum ack offset: sort offsets in descending order and pick (quorum_size - 1)th
        let mut offsets: Vec<u64> = s.match_offsets.values().copied().collect();
        offsets.sort_by(|a, b| b.cmp(a));

        if offsets.len() >= self.quorum_size {
            let quorum_ack = offsets[self.quorum_size - 1];
            if quorum_ack > s.high_watermark {
                s.high_watermark = quorum_ack;
            }
        }
        s.high_watermark
    }

    /// Record local append
    pub fn record_local_append(&self, new_leo: u64) {
        let mut s = self.state.write();
        let my_id = s.node_id;
        s.log_end_offset = new_leo;
        s.match_offsets.insert(my_id, new_leo);
    }

    pub fn role(&self) -> ReplicaRole {
        self.state.read().role
    }

    pub fn high_watermark(&self) -> u64 {
        self.state.read().high_watermark
    }

    pub fn current_term(&self) -> u64 {
        self.state.read().current_term
    }
}
