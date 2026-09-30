use roaring::RoaringBitmap;

pub enum FilterStrategy {
    BruteForce,
    InGraphFiltered,
    PostFilter,
}

pub struct FilterRouter;

impl FilterRouter {
    pub fn select_strategy(total_vectors: usize, matching_bitmap: &RoaringBitmap) -> FilterStrategy {
        let selectivity = (matching_bitmap.len() as f64) / (total_vectors as f64);
        if selectivity < 0.001 {
            FilterStrategy::BruteForce
        } else if selectivity < 0.15 {
            FilterStrategy::InGraphFiltered
        } else {
            FilterStrategy::PostFilter
        }
    }
}
