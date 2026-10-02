//! Consumer Groups & Cooperative Rebalancing (Features 08-079..08-091)

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentStrategy {
    Range,
    RoundRobin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub member_id: String,
    pub client_host: String,
    pub last_heartbeat: u64,
    pub assigned_partitions: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroup {
    pub group_id: String,
    pub topic: String,
    pub generation_id: u32,
    pub leader_id: Option<String>,
    pub members: HashMap<String, GroupMember>,
    pub committed_offsets: HashMap<u32, u64>, // partition_id -> committed_offset
    pub strategy: AssignmentStrategy,
}

/// 08-079..08-091: Consumer Group Coordinator
pub struct GroupCoordinator {
    groups: RwLock<HashMap<String, ConsumerGroup>>,
}

impl Default for GroupCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl GroupCoordinator {
    pub fn new() -> Self {
        Self {
            groups: RwLock::new(HashMap::new()),
        }
    }

    /// 08-079: Join consumer group
    pub fn join_group(&self, group_id: &str, topic: &str, member_id: &str, host: &str, now: u64) {
        let mut groups = self.groups.write();
        let group = groups.entry(group_id.to_string()).or_insert(ConsumerGroup {
            group_id: group_id.to_string(),
            topic: topic.to_string(),
            generation_id: 1,
            leader_id: Some(member_id.to_string()),
            members: HashMap::new(),
            committed_offsets: HashMap::new(),
            strategy: AssignmentStrategy::RoundRobin,
        });

        group.members.insert(
            member_id.to_string(),
            GroupMember {
                member_id: member_id.to_string(),
                client_host: host.to_string(),
                last_heartbeat: now,
                assigned_partitions: Vec::new(),
            },
        );

        if group.leader_id.is_none() {
            group.leader_id = Some(member_id.to_string());
        }
    }

    /// 08-080: Leave consumer group
    pub fn leave_group(&self, group_id: &str, member_id: &str) -> bool {
        let mut groups = self.groups.write();
        if let Some(group) = groups.get_mut(group_id) {
            group.members.remove(member_id);
            if group.leader_id.as_deref() == Some(member_id) {
                group.leader_id = group.members.keys().next().cloned();
            }
            true
        } else {
            false
        }
    }

    /// 08-081..08-085: Perform cooperative partition rebalance
    pub fn rebalance(&self, group_id: &str, total_partitions: u32) -> Result<u32, &'static str> {
        let mut groups = self.groups.write();
        let group = groups.get_mut(group_id).ok_or("Group not found")?;

        if group.members.is_empty() {
            return Ok(group.generation_id);
        }

        group.generation_id += 1;
        let mut member_keys: Vec<String> = group.members.keys().cloned().collect();
        member_keys.sort();

        // Clear existing assignments
        for m in group.members.values_mut() {
            m.assigned_partitions.clear();
        }

        match group.strategy {
            AssignmentStrategy::RoundRobin => {
                for p in 0..total_partitions {
                    let target_member = &member_keys[(p as usize) % member_keys.len()];
                    if let Some(member) = group.members.get_mut(target_member) {
                        member.assigned_partitions.push(p);
                    }
                }
            }
            AssignmentStrategy::Range => {
                let part_per_member = (total_partitions as usize + member_keys.len() - 1) / member_keys.len();
                for (idx, m_key) in member_keys.iter().enumerate() {
                    let start = (idx * part_per_member).min(total_partitions as usize) as u32;
                    let end = ((idx + 1) * part_per_member).min(total_partitions as usize) as u32;
                    if let Some(member) = group.members.get_mut(m_key) {
                        for p in start..end {
                            member.assigned_partitions.push(p);
                        }
                    }
                }
            }
        }

        Ok(group.generation_id)
    }

    /// 08-009: Commit offset
    pub fn commit_offset(&self, group_id: &str, partition: u32, offset: u64) -> Result<(), &'static str> {
        let mut groups = self.groups.write();
        let group = groups.get_mut(group_id).ok_or("Group not found")?;
        group.committed_offsets.insert(partition, offset);
        Ok(())
    }

    /// Fetch committed offset
    pub fn get_committed_offset(&self, group_id: &str, partition: u32) -> Option<u64> {
        let groups = self.groups.read();
        groups.get(group_id)?.committed_offsets.get(&partition).copied()
    }

    pub fn get_member_assignments(&self, group_id: &str, member_id: &str) -> Option<Vec<u32>> {
        let groups = self.groups.read();
        let group = groups.get(group_id)?;
        let member = group.members.get(member_id)?;
        Some(member.assigned_partitions.clone())
    }
}
