//! Distance metrics for high-dimensional vector search

use pf_vector_kit::{l2_distance_squared as pf_l2, dot_product as pf_dot, cosine_similarity as pf_cosine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    L2,
    Cosine,
    DotProduct,
}

#[inline]
pub fn euclidean_distance_squared(a: &[f32], b: &[f32]) -> f32 {
    pf_l2(a, b)
}

#[inline]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    pf_dot(a, b)
}

#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - pf_cosine(a, b)
}

#[inline]
pub fn compute_distance(a: &[f32], b: &[f32], metric: Metric) -> f32 {
    match metric {
        Metric::L2 => euclidean_distance_squared(a, b),
        Metric::Cosine => cosine_distance(a, b),
        Metric::DotProduct => -dot_product(a, b),
    }
}
