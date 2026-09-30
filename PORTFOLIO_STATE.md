# PORTFOLIO STATE (Single Source of Truth)

## 0. HEADER
- **session_no**: 4
- **last_run_timestamp**: 2026-09-30T18:04:54Z
- **last_commit**: 965b690d28f6e58427a433bbc9aee0b1ad29cc31
- **current_wave**: Wave 4 (Provenance & Verified P0 Slices)
- **monorepo_root**: /Users/avishjain/avishjain9-repos/portfolio
- **global_status**: AUDITED_AND_RECONCILED

---

## 1. ENVIRONMENT & GOVERNANCE RULES
- **rustc**: rustc 1.98.1 (48a229cea 2026-09-01)
- **cargo**: cargo 1.98.1 (797e8a9bc 2026-08-05)
- **python**: Python 3.13.9
- **node**: v22.23.2
- **gh**: gh version 2.101.0 (2026-09-15)
- **R1 (Evidence Integrity)**: Zero hand-typed numbers. All counts derived via tools/evidence-gen and tools/bench-collector.
- **R2 (Honesty Gate)**: Features without passing tests are marked Implemented or Scaffolded.
- **R3 (Zero Drift)**: Unified shared platform crates under `platform/rust/crates`.
- **R6 (Offline First)**: Full offline build, test, and verification capability.

---

## 2. PROJECT REGISTRY (20 PROJECTS)

| Slot | Project Slug | Total Specs | Verified P0 | Total Verified | GitHub Repository URL | Status |
|:---:|:---|:---:|:---:|:---:|:---|:---:|
| 01 | `01-rollback-netcode-server` | 90 | 3 | 4 | not pushed | AUDITED_VERIFIED |
| 02 | `02-hnsw-filtered-vector-index` | 152 | 4 | 4 | not pushed | AUDITED_VERIFIED |
| 03 | `03-bytecode-vm-gc` | 141 | 0 | 0 | not pushed | SPECIFIED |
| 04 | `04-columnar-query-engine` | 145 | 2 | 3 | not pushed | AUDITED_VERIFIED |
| 05 | `05-mini-quic` | 17 | 3 | 3 | not pushed | AUDITED_VERIFIED |
| 06 | `06-lsm-kv-engine` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 07 | `07-stream-detect-engine` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 08 | `08-pubsub-broker` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 09 | `09-distributed-cache` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 10 | `10-crdt-rich-text-editor` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 11 | `11-zanzibar-authz` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 12 | `12-ll-hls-origin` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 13 | `13-ledger-engine` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 14 | `14-tracing-backend` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 15 | `15-llm-inference-scheduler` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 16 | `16-rate-limit-gateway` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 17 | `17-cdn-caching-proxy` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 18 | `18-flash-sale-inventory` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 19 | `19-usage-billing-engine` | 17 | 0 | 0 | not pushed | SPECIFIED |
| 20 | `20-notification-delivery-platform` | 17 | 0 | 0 | not pushed | SPECIFIED |

---

## 3. PLATFORM INVENTORY (19 CRATES)

| Crate Name | Path | Purpose | Unit Tests | Fuzz/Loom/Bench |
|:---|:---|:---|:---:|:---:|
| `pf-bytes` | `platform/rust/crates/pf-bytes` | QuicVarInt & zero-copy buffer framing | 1 passed | none |
| `pf-checksum` | `platform/rust/crates/pf-checksum` | SIMD CRC32 & Blake3/XXHash kernels | 2 passed | none |
| `pf-collections` | `platform/rust/crates/pf-collections` | BloomFilter & zero-alloc RingBuffer | 2 passed | none |
| `pf-cache` | `platform/rust/crates/pf-cache` | S3-FIFO cache with Ghost queue eviction | 2 passed | none |
| `pf-sync` | `platform/rust/crates/pf-sync` | Lockless atomic ring queue & barriers | 2 passed | none |
| `pf-time` | `platform/rust/crates/pf-time` | Monotonic clock & simulation timestamps | 2 passed | none |
| `pf-sim` | `platform/rust/crates/pf-sim` | Deterministic network simulator (loss/reorder) | 2 passed | none |
| `pf-wal` | `platform/rust/crates/pf-wal` | Append-only write ahead log with CRC | 2 passed | none |
| `pf-storage-kit` | `platform/rust/crates/pf-storage-kit` | Block storage & chunk management | 2 passed | none |
| `pf-net` | `platform/rust/crates/pf-net` | UDP socket abstractions | 2 passed | none |
| `pf-crypto` | `platform/rust/crates/pf-crypto` | RFC 5869 / RFC 9001 HKDF & Blake3 MAC | 2 passed (RFC vectors) | none |
| `pf-ratelimit` | `platform/rust/crates/pf-ratelimit` | Lockless token bucket rate limiter | 2 passed | none |
| `pf-hashring` | `platform/rust/crates/pf-hashring` | Consistent hashing ring with vnodes | 2 passed | none |
| `pf-metrics` | `platform/rust/crates/pf-metrics` | HDR Histogram & latency trackers | 2 passed | none |
| `pf-http` | `platform/rust/crates/pf-http` | Unified Axum/Tower HTTP middleware | 2 passed | none |
| `pf-vector-kit` | `platform/rust/crates/pf-vector-kit` | SIMD L2 squared, Cosine, DotProduct | 1 passed | none |
| `pf-testkit` | `platform/rust/crates/pf-testkit` | Chaos testing & property generators | 2 passed | none |
| `pf-config` | `platform/rust/crates/pf-config` | Zero-dependency environment loader | 1 passed | none |
| `pf-errors` | `platform/rust/crates/pf-errors` | Shared error taxonomy & HTTP status codes | 1 passed | none |

---

## 4. PROJECT PROGRESS & QUALITY METRICS

| Project | Total Specs | Verified | Implemented | Scaffolded | Passing Tests | Criterion Latency / Throughput | Coverage |
|:---|:---:|:---:|:---:|:---:|:---:|:---|:---:|
| `01-rollback-netcode-server` | 90 | 4 | 86 | 0 | 18 passed | not yet measured | not yet measured |
| `02-hnsw-filtered-vector-index` | 152 | 4 | 63 | 85 | 5 passed | 198.31 µs | not yet measured |
| `03-bytecode-vm-gc` | 141 | 0 | 0 | 141 | not yet measured | not yet measured | not yet measured |
| `04-columnar-query-engine` | 145 | 3 | 43 | 99 | 3 passed | 115.37 µs | not yet measured |
| `05-mini-quic` | 17 | 3 | 1 | 13 | 5 passed | 75.416 ns | not yet measured |
| `06-lsm-kv-engine` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `07-stream-detect-engine` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `08-pubsub-broker` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `09-distributed-cache` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `10-crdt-rich-text-editor` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `11-zanzibar-authz` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `12-ll-hls-origin` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `13-ledger-engine` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `14-tracing-backend` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `15-llm-inference-scheduler` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `16-rate-limit-gateway` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `17-cdn-caching-proxy` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `18-flash-sale-inventory` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `19-usage-billing-engine` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |
| `20-notification-delivery-platform` | 17 | 0 | 0 | 17 | not yet measured | not yet measured | not yet measured |

---

## 5. MEASURED BENCHMARK RESULTS & EVIDENCE TRACEABILITY

| Benchmark Result File | Project | Command | Git Commit | Measurements Summary |
|:---|:---|:---|:---|:---|
| `hnsw_benchmarks.json` | `02-hnsw-filtered-vector-index` | `cargo bench` | `965b690d28` | collected |
| `hnsw_filtered_vector_index_benchmarks.json` | `02-hnsw-filtered-vector-index` | `cargo bench --manifest-path /Users/avishjain/avishjain9-repos/portfolio/projects/02-hnsw-filtered-vector-index/backend/Cargo.toml --bench hnsw_bench -- --baseline new` | `965b690d28` | benchmark: 198.31 µs |
| `columnar_benchmarks.json` | `04-columnar-query-engine` | `cargo bench` | `965b690d28` | collected |
| `columnar_query_engine_benchmarks.json` | `04-columnar-query-engine` | `cargo bench --manifest-path /Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/Cargo.toml --bench columnar_bench -- --baseline new` | `965b690d28` | benchmark: 115.37 µs |
| `mini_quic_benchmarks.json` | `05-mini-quic` | `cargo bench --manifest-path /Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/Cargo.toml --bench quic_bench -- --baseline new` | `965b690d28` | benchmark: 75.416 ns |
| `quic_benchmarks.json` | `05-mini-quic` | `cargo bench` | `2d35bd2f97` | collected |

---

## 6. ARCHITECTURAL DECISIONS SUMMARY
- **ADR-001 - ADR-016**: Recorded in `docs/DECISIONS.md`.

---

## 7. GAPS & AUDIT FINDINGS
- **Independent Ingest Parity**: All 20 workbooks in `inputs/specs/` match `features.jsonl` row counts exactly (automated test `test_feature_ingest.py` passing).
- **HNSW Recall**: Algorithm 4 heuristic diversity selection achieved 100% empirical Recall@10 at ef=128 on seeded 2k vectors.
- **RFC 9001 Key Derivation**: Verified RFC 5869 and RFC 9001 Appendix A.1 test vectors in `pf-crypto`.

---

## 8. MANUAL TASKS
- Nightly toolchain installation (`rustup toolchain install nightly`) for running `cargo-fuzz` targets.
- Docker daemon startup for Postgres/Redis service containers if external integration tests are executed.

---

## 9. REUSE & MONOREPO CONFORMANCE
- **Code Deduplication Scan**: 0 cross-project code duplications (>30 lines) detected.
- **Platform Primitive Conformance**: 0 private primitive reimplementations detected.

---

## 10. NEXT WAVE ACTION PLAN
1. Run `make ci` on clean clone and save output to `evidence/ci_run.txt`.
2. Add Parquet file format roundtrip for `04-columnar-query-engine`.
3. Add `pf-sim` network loss transfer benchmark for `05-mini-quic`.

---

## 11. RESUME INSTRUCTIONS
1. Pull latest workspace state: commit `{git_commit}`.
2. Run `python3 tools/evidence_gen.py` to regenerate verified evidence rows.
3. Run `python3 tools/bench_collector.py` to refresh Criterion benchmarks.

---

## PROMPTS_I_NEED
1. BBR vs Cubic pacing simulation test harness for `05-mini-quic` under bursty loss.
2. Vectorized SIMD chunk aggregation & hash-join engine for `04-columnar-query-engine`.
3. Auto-tuned HNSW heuristic neighbor pruning for >95% recall@10 on 128-dim embeddings.
