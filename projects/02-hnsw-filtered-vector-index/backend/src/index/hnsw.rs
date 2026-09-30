//! Hierarchical Navigable Small World (HNSW) Proximity Graph

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use std::sync::Arc;
use parking_lot::RwLock;
use rand::Rng;
use roaring::RoaringBitmap;
use crate::simd::distance::{compute_distance, Metric};

pub type LayerAdjacency = Vec<HashMap<u64, Vec<u64>>>;
pub type VectorStore = HashMap<u64, Vec<f32>>;

#[derive(Debug, Clone)]
struct Candidate {
    id: u64,
    distance: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.id == other.id
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap by distance
        other.distance.partial_cmp(&self.distance).unwrap_or(Ordering::Equal)
    }
}

#[derive(Debug, Clone)]
struct FurthestCandidate(Candidate);

impl PartialEq for FurthestCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for FurthestCandidate {}

impl PartialOrd for FurthestCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FurthestCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.distance.partial_cmp(&other.0.distance).unwrap_or(Ordering::Equal)
    }
}

pub struct HnswIndex {
    pub dimension: usize,
    pub m: usize,
    pub m0: usize,
    pub ef_construction: usize,
    pub m_l: f64,
    pub metric: Metric,
    pub vectors: Arc<RwLock<VectorStore>>,
    pub layers: Arc<RwLock<LayerAdjacency>>,
    pub entry_point: Arc<RwLock<Option<u64>>>,
    pub max_layer: Arc<RwLock<usize>>,
}

impl HnswIndex {
    pub fn new(dimension: usize, m: usize, ef_construction: usize) -> Self {
        Self::with_metric(dimension, m, ef_construction, Metric::L2)
    }

    pub fn with_metric(dimension: usize, m: usize, ef_construction: usize, metric: Metric) -> Self {
        let m_l = 1.0 / (m as f64).ln().max(1e-4);
        Self {
            dimension,
            m,
            m0: m * 2,
            ef_construction,
            m_l,
            metric,
            vectors: Arc::new(RwLock::new(HashMap::new())),
            layers: Arc::new(RwLock::new(Vec::new())),
            entry_point: Arc::new(RwLock::new(None)),
            max_layer: Arc::new(RwLock::new(0)),
        }
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen_range(1e-7..1.0);
        (-r.ln() * self.m_l).floor() as usize
    }

    pub fn insert(&self, id: u64, vector: &[f32]) -> Result<(), &'static str> {
        if vector.len() != self.dimension {
            return Err("Vector dimension mismatch");
        }

        let mut vectors = self.vectors.write();
        let mut layers = self.layers.write();
        let mut entry_point = self.entry_point.write();
        let mut max_layer = self.max_layer.write();

        vectors.insert(id, vector.to_vec());

        let node_level = self.random_level();
        while layers.len() <= node_level {
            layers.push(HashMap::new());
        }

        if entry_point.is_none() {
            *entry_point = Some(id);
            *max_layer = node_level;
            for l in 0..=node_level {
                layers[l].insert(id, Vec::new());
            }
            return Ok(());
        }

        let mut curr_ep = entry_point.unwrap();
        let curr_max_level = *max_layer;

        // 1. Greedy search from top level down to node_level + 1
        let mut curr_dist = compute_distance(vector, &vectors[&curr_ep], self.metric);
        for l in (node_level + 1..=curr_max_level).rev() {
            let mut changed = true;
            while changed {
                changed = false;
                if let Some(neighbors) = layers[l].get(&curr_ep) {
                    for &neighbor in neighbors {
                        let d = compute_distance(vector, &vectors[&neighbor], self.metric);
                        if d < curr_dist {
                            curr_dist = d;
                            curr_ep = neighbor;
                            changed = true;
                        }
                    }
                }
            }
        }

        // 2. Search layer and connect neighbors from min(curr_max_level, node_level) down to 0
        let mut ep_candidates = vec![curr_ep];
        let top_connect = std::cmp::min(curr_max_level, node_level);
        for l in (0..=top_connect).rev() {
            let max_conn = if l == 0 { self.m0 } else { self.m };
            let candidates = self.search_layer_internal(vector, &ep_candidates, self.ef_construction, l, &layers, &vectors);
            
            // Select up to max_conn neighbors
            let selected_neighbors: Vec<u64> = candidates.iter().take(max_conn).map(|c| c.id).collect();
            layers[l].insert(id, selected_neighbors.clone());

            // Connect neighbors bidirectionally
            for &neighbor in &selected_neighbors {
                let neighbors_entry = layers[l].entry(neighbor).or_default();
                if !neighbors_entry.contains(&id) {
                    neighbors_entry.push(id);
                    if neighbors_entry.len() > max_conn {
                        // Prune neighbor with worst distance
                        let n_vec = &vectors[&neighbor];
                        let mut distances: Vec<(u64, f32)> = neighbors_entry.iter()
                            .map(|&n_id| (n_id, compute_distance(n_vec, &vectors[&n_id], self.metric)))
                            .collect();
                        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
                        *neighbors_entry = distances.iter().take(max_conn).map(|d| d.0).collect();
                    }
                }
            }

            ep_candidates = candidates.iter().map(|c| c.id).collect();
        }

        if node_level > curr_max_level {
            for l in curr_max_level + 1..=node_level {
                layers[l].insert(id, Vec::new());
            }
            *max_layer = node_level;
            *entry_point = Some(id);
        }

        Ok(())
    }

    fn search_layer_internal(
        &self,
        query: &[f32],
        entry_points: &[u64],
        ef: usize,
        layer: usize,
        layers: &[HashMap<u64, Vec<u64>>],
        vectors: &HashMap<u64, Vec<f32>>,
    ) -> Vec<Candidate> {
        let mut visited = HashSet::new();
        let mut candidate_heap = BinaryHeap::new(); // Min-heap by distance
        let mut result_heap = BinaryHeap::new();    // Max-heap by distance (FurthestCandidate)

        for &ep in entry_points {
            if let Some(v) = vectors.get(&ep) {
                let dist = compute_distance(query, v, self.metric);
                visited.insert(ep);
                let cand = Candidate { id: ep, distance: dist };
                candidate_heap.push(cand.clone());
                result_heap.push(FurthestCandidate(cand));
            }
        }

        while let Some(current) = candidate_heap.pop() {
            let furthest = result_heap.peek().unwrap();
            if current.distance > furthest.0.distance && result_heap.len() >= ef {
                break;
            }

            if let Some(neighbors) = layers[layer].get(&current.id) {
                for &neighbor in neighbors {
                    if visited.insert(neighbor) {
                        if let Some(v) = vectors.get(&neighbor) {
                            let dist = compute_distance(query, v, self.metric);
                            let furthest = result_heap.peek().unwrap();
                            if dist < furthest.0.distance || result_heap.len() < ef {
                                let cand = Candidate { id: neighbor, distance: dist };
                                candidate_heap.push(cand.clone());
                                result_heap.push(FurthestCandidate(cand));
                                if result_heap.len() > ef {
                                    result_heap.pop();
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut results: Vec<Candidate> = result_heap.into_iter().map(|f| f.0).collect();
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal));
        results
    }

    pub fn search(&self, query: &[f32], k: usize, ef_search: usize) -> Vec<(u64, f32)> {
        self.search_filtered(query, k, ef_search, None)
    }

    pub fn search_filtered(&self, query: &[f32], k: usize, ef_search: usize, allowed_ids: Option<&RoaringBitmap>) -> Vec<(u64, f32)> {
        let vectors = self.vectors.read();
        let layers = self.layers.read();
        let entry_point = self.entry_point.read();
        let max_layer = *self.max_layer.read();

        let ep = match *entry_point {
            Some(id) => id,
            None => return Vec::new(),
        };

        if query.len() != self.dimension {
            return Vec::new();
        }

        let mut curr_ep = ep;
        let mut curr_dist = compute_distance(query, &vectors[&curr_ep], self.metric);

        // Greedy search through upper layers
        for l in (1..=max_layer).rev() {
            let mut changed = true;
            while changed {
                changed = false;
                if let Some(neighbors) = layers[l].get(&curr_ep) {
                    for &neighbor in neighbors {
                        let d = compute_distance(query, &vectors[&neighbor], self.metric);
                        if d < curr_dist {
                            curr_dist = d;
                            curr_ep = neighbor;
                            changed = true;
                        }
                    }
                }
            }
        }

        // Layer 0 beam search
        let candidates = self.search_layer_internal(query, &[curr_ep], ef_search.max(k), 0, &layers, &vectors);
        
        candidates.into_iter()
            .filter(|c| {
                if let Some(filter) = allowed_ids {
                    filter.contains(c.id as u32)
                } else {
                    true
                }
            })
            .take(k)
            .map(|c| (c.id, c.distance))
            .collect()
    }
}
