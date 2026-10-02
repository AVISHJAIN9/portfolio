use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lsm_kv_engine::engine::LsmEngine;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tempfile::tempdir;

fn bench_fillseq(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsm_fillseq");
    group.sample_size(10);

    group.bench_function("fillseq_10k_entries", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let engine = LsmEngine::open(dir.path()).unwrap();
            for i in 0..10_000 {
                let key = format!("key_{:08}", i).into_bytes();
                let val = format!("val_{:08}", i).into_bytes();
                engine.put(black_box(&key), black_box(&val)).unwrap();
            }
        });
    });
    group.finish();
}

fn bench_fillrandom(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsm_fillrandom");
    group.sample_size(10);

    group.bench_function("fillrandom_10k_entries", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let engine = LsmEngine::open(dir.path()).unwrap();
            let mut rng = StdRng::seed_from_u64(42);
            for _ in 0..10_000 {
                let k_num: u64 = rng.gen_range(0..1_000_000);
                let key = format!("rand_key_{:08}", k_num).into_bytes();
                let val = format!("rand_val_{:08}", k_num).into_bytes();
                engine.put(black_box(&key), black_box(&val)).unwrap();
            }
        });
    });
    group.finish();
}

fn bench_readrandom(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsm_readrandom");
    group.sample_size(10);

    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    let mut rng = StdRng::seed_from_u64(1337);
    for i in 0..10_000 {
        let key = format!("key_{:08}", i).into_bytes();
        let val = format!("val_{:08}", i).into_bytes();
        engine.put(&key, &val).unwrap();
    }
    engine.flush().unwrap();

    group.bench_function("readrandom_10k_lookups", |b| {
        b.iter(|| {
            let k_idx = rng.gen_range(0..10_000);
            let key = format!("key_{:08}", k_idx).into_bytes();
            let res = engine.get(black_box(&key)).unwrap();
            black_box(res);
        });
    });
    group.finish();
}

fn bench_overwrite(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsm_overwrite");
    group.sample_size(10);

    let dir = tempdir().unwrap();
    let engine = LsmEngine::open(dir.path()).unwrap();
    for i in 0..10_000 {
        let key = format!("key_{:08}", i).into_bytes();
        let val = format!("val_{:08}", i).into_bytes();
        engine.put(&key, &val).unwrap();
    }

    group.bench_function("overwrite_10k_entries", |b| {
        b.iter(|| {
            for i in 0..10_000 {
                let key = format!("key_{:08}", i).into_bytes();
                let val = format!("updated_val_{:08}", i).into_bytes();
                engine.put(black_box(&key), black_box(&val)).unwrap();
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_fillseq, bench_fillrandom, bench_readrandom, bench_overwrite);
criterion_main!(benches);

