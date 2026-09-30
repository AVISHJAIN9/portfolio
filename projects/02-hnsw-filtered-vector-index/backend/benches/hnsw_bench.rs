use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hnsw_filtered_vector_index::index::hnsw::HnswIndex;
use rand::Rng;

fn bench_hnsw_search(c: &mut Criterion) {
    let index = HnswIndex::new(32, 16, 64);
    let mut rng = rand::thread_rng();

    // Insert 1,000 32-dimensional vectors
    for id in 0..1000 {
        let vec: Vec<f32> = (0..32).map(|_| rng.gen_range(-1.0..1.0)).collect();
        index.insert(id, &vec).unwrap();
    }

    let query: Vec<f32> = (0..32).map(|_| rng.gen_range(-1.0..1.0)).collect();

    c.bench_function("hnsw_search_top10_ef32", |b| {
        b.iter(|| {
            let res = index.search(black_box(&query), black_box(10), black_box(32));
            black_box(res.len());
        });
    });
}

criterion_group!(benches, bench_hnsw_search);
criterion_main!(benches);
