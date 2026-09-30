//! Adaptive filter routing for vector queries

use roaring::RoaringBitmap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterStrategy {
    BruteForce,
    InGraphFiltered,
    PostFilter,
}

pub struct FilterRouter;

impl FilterRouter {
    pub fn select_strategy(total_vectors: usize, matching_bitmap: &RoaringBitmap) -> FilterStrategy {
        if total_vectors == 0 {
            return FilterStrategy::BruteForce;
        }
        let selectivity = (matching_bitmap.len() as f64) / (total_vectors as f64);
        if selectivity < 0.005 {
            FilterStrategy::BruteForce
        } else if selectivity < 0.20 {
            FilterStrategy::InGraphFiltered
        } else {
            FilterStrategy::PostFilter
        }
    }
}
