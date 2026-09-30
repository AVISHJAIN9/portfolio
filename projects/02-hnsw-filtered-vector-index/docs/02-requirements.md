# 02 - Requirements Specification: HNSW Filtered Vector Index

## 1. Functional Requirements [Specified]
- **FR-001**: Construct multi-layer HNSW graph with configurable $M$ (max links per node) and $efConstruction$ (construction beam width).
- **FR-002**: Implement auto-vectorized SIMD kernels for Squared L2 Distance, Inner Product, and Cosine Similarity across x86_64 (AVX2/FMA) and ARM64 (NEON).
- **FR-003**: Provide Int8 Scalar Quantization compressing 32-bit floats into signed 8-bit integers with per-vector scale and offset, plus top-K Float32 rescoring.
- **FR-004**: Implement three distinct filter strategies:
  1. *In-Graph Filtered Search*: Skips non-matching nodes during graph traversal.
  2. *Post-Filtering*: Expands $efSearch$ adaptively and discards non-matching nodes from candidate queue.
  3. *Brute-Force Scan*: Directly scans Roaring Bitmap matching sets when selectivity $< 0.1\%$.
- **FR-005**: Support concurrent read-write access using lock striping (striped `parking_lot::RwLock` across graph nodes).
- **FR-006**: Support soft-deletions using Roaring Bitmap tombstones with background heuristic graph edge repair.
- **FR-007**: Persist index state via append-only Write-Ahead Log (WAL) and zero-copy Memory-Mapped (`mmap`) snapshot checkpoints.
- **FR-008 (Tier A AIML)**: Execute a learned query planner predicting the fastest filter execution strategy based on predicate selectivity, vector dimensionality, and target $K$.
- **FR-009 (Tier B AIML)**: Provide natural language to structured filter JSON generation with schema validation.
