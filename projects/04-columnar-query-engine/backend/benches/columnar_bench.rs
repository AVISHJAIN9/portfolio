use criterion::{black_box, criterion_group, criterion_main, Criterion};
use columnar_query_engine::storage::chunk::ColumnChunk;
use columnar_query_engine::execution::vector::{SelectionVector, VectorizedExecutor, Predicate};

fn bench_chunk_filter(c: &mut Criterion) {
    let values: Vec<i64> = (0..100_000).collect();
    let chunk = ColumnChunk::new_int64("id", values);
    let pred = Predicate::Gt("id".to_string(), 50_000);
    
    c.bench_function("columnar_filter_scan_100k_rows", |b| {
        let selection = SelectionVector::all(100_000);
        b.iter(|| {
            let res = VectorizedExecutor::evaluate_filter(
                black_box(&chunk),
                black_box(&pred),
                black_box(&selection),
            );
            black_box(res.len());
        });
    });
}

criterion_group!(benches, bench_chunk_filter);
criterion_main!(benches);
