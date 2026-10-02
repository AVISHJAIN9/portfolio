use columnar_query_engine::execution::vector::{Predicate, SelectionVector, VectorizedExecutor};
use columnar_query_engine::storage::chunk::ColumnChunk;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

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

fn bench_tpch_q6_filtered_agg(c: &mut Criterion) {
    let num_rows = 100_000;
    let prices: Vec<i64> = (0..num_rows).map(|i| (i % 1000) as i64).collect();
    let chunk = ColumnChunk::new_int64("extendedprice", prices);
    let pred = Predicate::Gt("extendedprice".to_string(), 500);

    c.bench_function("tpch_q6_vectorized_100k", |b| {
        let selection = SelectionVector::all(num_rows);
        b.iter(|| {
            let filtered_sel = VectorizedExecutor::evaluate_filter(
                black_box(&chunk),
                black_box(&pred),
                black_box(&selection),
            );
            let sum: i64 = match &chunk.data {
                columnar_query_engine::storage::chunk::ColumnVector::Int64(v) => {
                    filtered_sel.indices.iter().map(|&idx| v[idx]).sum()
                }
                _ => 0,
            };
            black_box(sum);
        });
    });
}

fn bench_tpch_q1_grouped_agg(c: &mut Criterion) {
    let num_rows = 100_000;
    let quantities: Vec<i64> = (0..num_rows).map(|i| (i % 50) as i64).collect();
    let chunk = ColumnChunk::new_int64("quantity", quantities);
    let pred = Predicate::Gt("quantity".to_string(), 24);

    c.bench_function("tpch_q1_vectorized_100k", |b| {
        let selection = SelectionVector::all(num_rows);
        b.iter(|| {
            let filtered_sel = VectorizedExecutor::evaluate_filter(
                black_box(&chunk),
                black_box(&pred),
                black_box(&selection),
            );
            black_box(filtered_sel.len());
        });
    });
}

criterion_group!(
    benches,
    bench_chunk_filter,
    bench_tpch_q6_filtered_agg,
    bench_tpch_q1_grouped_agg
);
criterion_main!(benches);
