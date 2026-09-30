# 01 - Vision & Scope: HNSW Filtered Vector Index

## 1. Executive Summary [Specified]
High-dimensional approximate nearest neighbor (ANN) search is the core retrieval primitive powering modern generative AI, retrieval-augmented generation (RAG), and semantic recommendation systems. However, real-world production search requires arbitrary boolean metadata filtering (e.g. `price < 50 AND category == 'electronics'`), zero-downtime updates, soft deletes, and extreme memory efficiency.

`hnsw-filtered-vector-index` is an industrial-grade, from-scratch vector index written in Rust. It implements a multi-layer Hierarchical Navigable Small World (HNSW) graph, AVX2/NEON SIMD distance kernels, Int8 scalar quantization with dynamic rescoring, striped read-write locks for concurrent mutation, tombstone deletions with lazy repair, and a three-strategy adaptive filter router (in-graph filtered beam search, post-filtering, and brute-force fallback).

## 2. Target Performance & Metrics [Specified]
| Metric | Specification Target | Status |
| :--- | :--- | :--- |
| Unfiltered Recall@10 (SIFT1M) | >= 0.985 with efSearch=64 | **TO BE MEASURED** |
| Filtered Recall@10 (at 5% selectivity) | >= 0.950 | **TO BE MEASURED** |
| QPS per CPU Core (Float32, dim=128) | >= 4,500 queries/sec | **TO BE MEASURED** |
| QPS per CPU Core (Int8 Quantized) | >= 12,000 queries/sec | **TO BE MEASURED** |
| Memory Reduction via Int8 Quantization | 4x reduction (1 byte/dim vs 4 bytes/dim) | [Specified] |
| Write Throughput (Concurrent Insert) | >= 25,000 vectors/sec | **TO BE MEASURED** |
| Crash Recovery Time (1M Vectors) | < 1.5 seconds via mmap | **TO BE MEASURED** |
