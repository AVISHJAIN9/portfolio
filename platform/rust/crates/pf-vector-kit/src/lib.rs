/// Computes squared Euclidean L2 distance between two f32 slices.
#[inline]
pub fn l2_distance_squared(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vector dimension mismatch");
    let mut sum = 0.0f32;
    // Auto-vectorizable unrolled loop by LLVM
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum += diff * diff;
    }
    sum
}

/// Computes Dot Product between two f32 slices.
#[inline]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vector dimension mismatch");
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

/// Computes Cosine Similarity between two f32 slices.
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product(a, b);
    let norm_a = dot_product(a, a).sqrt();
    let norm_b = dot_product(b, b).sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_kernels() {
        let v1 = [1.0, 2.0, 3.0, 4.0];
        let v2 = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(l2_distance_squared(&v1, &v2), 0.0);
        assert_eq!(dot_product(&v1, &v2), 30.0);
        assert!((cosine_similarity(&v1, &v2) - 1.0).abs() < 1e-6);
    }
}
