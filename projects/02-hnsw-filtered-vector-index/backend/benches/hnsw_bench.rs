use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hnsw_filtered_vector_index::index::hnsw::HnswIndex;
use hnsw_filtered_vector_index::simd::distance::compute_distance;
use hnsw_filtered_vector_index::simd::distance::Metric;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

fn bench_hnsw_100k_eval(c: &mut Criterion) {
    let dim = 128;
    let n_vectors = 10_000; // Fast benchmark loop with full recall calculation
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let index = HnswIndex::new(dim, 16, 64);
    let mut all_vectors = Vec::with_capacity(n_vectors);

    let start_build = Instant::now();
    for id in 0..n_vectors {
        let vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        index.insert(id as u64, &vec).unwrap();
        all_vectors.push(vec);
    }
    let build_time = start_build.elapsed();

    // Generate 100 query vectors and compute ground-truth brute force
    let n_queries = 50;
    let mut queries = Vec::with_capacity(n_queries);
    let mut ground_truth = Vec::with_capacity(n_queries);

    for _ in 0..n_queries {
        let q: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let mut brute_distances: Vec<(u64, f32)> = all_vectors
            .iter()
            .enumerate()
            .map(|(id, v)| (id as u64, compute_distance(&q, v, Metric::L2)))
            .collect();
        brute_distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let top10_gt: Vec<u64> = brute_distances.iter().take(10).map(|d| d.0).collect();
        
        queries.push(q);
        ground_truth.push(top10_gt);
    }

    // Evaluate empirical recall@10
    let mut total_hits = 0;
    for (q, gt) in queries.iter().zip(ground_truth.iter()) {
        let hnsw_res = index.search(q, 10, 64);
        let hnsw_ids: Vec<u64> = hnsw_res.iter().map(|r| r.0).collect();
        for id in &hnsw_ids {
            if gt.contains(id) {
                total_hits += 1;
            }
        }
    }
    let recall_at_10 = (total_hits as f64) / ((n_queries * 10) as f64);
    eprintln!("\n=== HNSW Empirical Evaluation (Seed=42) ===");
    eprintln!("  Dataset: N={}, Dim={}", n_vectors, dim);
    eprintln!("  Build Time: {:.2?}", build_time);
    eprintln!("  Empirical Recall@10: {:.4} ({:.2}%)", recall_at_10, recall_at_10 * 100.0);

    let query = queries[0].clone();
    c.bench_function("hnsw_search_128dim_ef64", |b| {
        b.iter(|| {
            let res = index.search(black_box(&query), black_box(10), black_box(64));
            black_box(res.len());
        });
    });
}

criterion_group!(benches, bench_hnsw_100k_eval);
criterion_main!(benches);
