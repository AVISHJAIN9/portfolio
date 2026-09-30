use hnsw_filtered_vector_index::index::hnsw::HnswIndex;
use hnsw_filtered_vector_index::quantization::scalar::ScalarQuantizer;
use hnsw_filtered_vector_index::storage::wal::{VectorWal, WalRecord};
use roaring::RoaringBitmap;

// FEAT: FEAT-02-017
#[test]
fn test_hnsw_insertion_and_search() {
    let index = HnswIndex::new(4, 16, 64);
    
    let v1 = [1.0, 0.0, 0.0, 0.0];
    let v2 = [0.0, 1.0, 0.0, 0.0];
    let v3 = [0.9, 0.1, 0.0, 0.0];
    let v4 = [0.0, 0.0, 1.0, 0.0];

    index.insert(1, &v1).unwrap();
    index.insert(2, &v2).unwrap();
    index.insert(3, &v3).unwrap();
    index.insert(4, &v4).unwrap();

    let query = [1.0, 0.0, 0.0, 0.0];
    let results = index.search(&query, 2, 32);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, 1);
    assert_eq!(results[1].0, 3);
}

// FEAT: FEAT-02-019
#[test]
fn test_hnsw_filtered_search() {
    let index = HnswIndex::new(4, 16, 64);

    for i in 0..50 {
        let v = [i as f32, (i * 2) as f32, 0.0, 1.0];
        index.insert(i as u64, &v).unwrap();
    }

    let mut allowed = RoaringBitmap::new();
    allowed.insert(10);
    allowed.insert(20);
    allowed.insert(30);

    let query = [10.0, 20.0, 0.0, 1.0];
    let results = index.search_filtered(&query, 5, 32, Some(&allowed));

    assert!(!results.is_empty());
    for (id, _) in &results {
        assert!(allowed.contains(*id as u32));
    }
    assert_eq!(results[0].0, 10);
}

// FEAT: FEAT-02-066
#[test]
fn test_scalar_quantization_roundtrip() {
    let quantizer = ScalarQuantizer::new(-1.0, 1.0);
    let original = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
    let quantized = quantizer.quantize(&original);
    let dequantized = quantizer.dequantize(&quantized);

    for (orig, deq) in original.iter().zip(dequantized.iter()) {
        assert!((orig - deq).abs() < 0.02);
    }
}

// FEAT: FEAT-02-017
#[test]
fn test_wal_persistence() {
    let path = "/tmp/test_vector_wal.log";
    let _ = std::fs::remove_file(path);

    {
        let mut wal = VectorWal::open(path).unwrap();
        wal.append(&WalRecord::Insert {
            id: 42,
            vector: vec![1.0, 2.0, 3.0, 4.0],
            metadata: vec!["tag:test".into()],
        }).unwrap();
    }

    let records = VectorWal::replay(path).unwrap();
    assert_eq!(records.len(), 1);
    if let WalRecord::Insert { id, vector, .. } = &records[0] {
        assert_eq!(*id, 42);
        assert_eq!(vector.len(), 4);
    } else {
        panic!("Expected Insert record");
    }

    let _ = std::fs::remove_file(path);
}

// FEAT: FEAT-02-018
#[test]
fn test_hnsw_high_recall_acceptance() {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;
    use hnsw_filtered_vector_index::simd::distance::{compute_distance, Metric};

    let dim = 32;
    let n_vectors = 2_000;
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let index = HnswIndex::new(dim, 16, 128);
    let mut all_vectors = Vec::with_capacity(n_vectors);

    for id in 0..n_vectors {
        let vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        index.insert(id as u64, &vec).unwrap();
        all_vectors.push(vec);
    }

    let n_queries = 30;
    let mut total_hits = 0;

    for _ in 0..n_queries {
        let q: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let mut brute: Vec<(u64, f32)> = all_vectors
            .iter()
            .enumerate()
            .map(|(id, v)| (id as u64, compute_distance(&q, v, Metric::L2)))
            .collect();
        brute.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let top10_gt: Vec<u64> = brute.iter().take(10).map(|d| d.0).collect();

        let hnsw_res = index.search(&q, 10, 128);
        let hnsw_ids: Vec<u64> = hnsw_res.iter().map(|r| r.0).collect();

        for id in &hnsw_ids {
            if top10_gt.contains(id) {
                total_hits += 1;
            }
        }
    }

    let recall = (total_hits as f64) / ((n_queries * 10) as f64);
    assert!(recall >= 0.90, "Recall@10 must be >= 90%, got {:.2}%", recall * 100.0);
}
