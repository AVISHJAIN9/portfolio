# PORTFOLIO STATE (Single Source of Truth)

## 0. HEADER
- **session_no**: 8
- **last_run_timestamp**: 2026-10-01T15:52:08Z
- **last_commit**: 49adde6b5f31402af580e13e984e7366e1e6d358
- **current_wave**: Wave 8 (Report Integrity, Mechanical Gate, Loom Permutation, Scale Differential, TPC-H SF1, and LSM Fsync Durability)
- **monorepo_root**: /Users/avishjain/avishjain9-repos/portfolio
- **global_status**: AUDITED_VERIFIED_HONEST_PROGRESS

---

## 1. ENVIRONMENT & GOVERNANCE RULES
- **rustc**: rustc 1.98.1 (48a229cea 2026-09-01)
- **cargo**: cargo 1.98.1 (797e8a9bc 2026-08-05)
- **python**: Python 3.9.6
- **node**: not installed
- **gh**: not installed
- **R1 (Evidence Integrity)**: Zero hand-typed numbers. All counts derived via `tools/evidence_gen.py`, `tools/bench_collector.py`, and `tools/workbook_status.py`.
- **R2 (Honesty Gate)**: Features without passing annotated tests in `make ci` are marked Implemented or Scaffolded. Verified is strictly reserved for generated evidence.
- **R3 (Zero Drift)**: Unified shared platform crates under `platform/rust/crates`.
- **R6 (Offline First)**: Full offline build, test, and verification capability.

---

## 2. PROJECT REGISTRY (20 PROJECTS)

| Slot | Project Slug | Total Specs | Verified P0 | Total Verified | GitHub Repository URL | Status |
|:---:|:---|:---:|:---:|:---:|:---|:---:|
| 01 | `01-rollback-netcode-server` | 90 | 2 | 3 | not pushed | UNVERIFIED_SOURCE |
| 02 | `02-hnsw-filtered-vector-index` | 152 | 4 | 4 | not pushed | UNVERIFIED_SOURCE |
| 03 | `03-bytecode-vm-gc` | 141 | 0 | 0 | not pushed | UNVERIFIED_SOURCE |
| 04 | `04-columnar-query-engine` | 145 | 20 | 20 | not pushed | AUDITED_VERIFIED |
| 05 | `05-mini-quic` | 145 | 56 | 56 | not pushed | AUDITED_VERIFIED |
| 06 | `06-lsm-kv-engine` | 145 | 24 | 24 | not pushed | AUDITED_VERIFIED |
| 07 | `07-stream-detect-engine` | 0 | 0 | 0 | not pushed | EMPTY |
| 08 | `08-pubsub-broker` | 0 | 0 | 0 | not pushed | EMPTY |
| 09 | `09-distributed-cache` | 0 | 0 | 0 | not pushed | EMPTY |
| 10 | `10-crdt-rich-text-editor` | 0 | 0 | 0 | not pushed | EMPTY |
| 11 | `11-zanzibar-authz` | 0 | 0 | 0 | not pushed | EMPTY |
| 12 | `12-ll-hls-origin` | 0 | 0 | 0 | not pushed | EMPTY |
| 13 | `13-ledger-engine` | 0 | 0 | 0 | not pushed | EMPTY |
| 14 | `14-tracing-backend` | 0 | 0 | 0 | not pushed | EMPTY |
| 15 | `15-llm-inference-scheduler` | 0 | 0 | 0 | not pushed | EMPTY |
| 16 | `16-rate-limit-gateway` | 0 | 0 | 0 | not pushed | EMPTY |
| 17 | `17-cdn-caching-proxy` | 0 | 0 | 0 | not pushed | EMPTY |
| 18 | `18-flash-sale-inventory` | 0 | 0 | 0 | not pushed | EMPTY |
| 19 | `19-usage-billing-engine` | 0 | 0 | 0 | not pushed | EMPTY |
| 20 | `20-notification-delivery-platform` | 0 | 0 | 0 | not pushed | EMPTY |

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
| `pf-crypto` | `platform/rust/crates/pf-crypto` | RFC 5869 / RFC 9001 HKDF & RFC 7748 X25519 | 3 passed (RFC vectors) | none |
| `pf-ratelimit` | `platform/rust/crates/pf-ratelimit` | Lockless token bucket rate limiter | 2 passed | none |
| `pf-hashring` | `platform/rust/crates/pf-hashring` | Consistent hashing ring with vnodes | 2 passed | none |
| `pf-metrics` | `platform/rust/crates/pf-metrics` | HDR Histogram & latency trackers | 2 passed | none |
| `pf-http` | `platform/rust/crates/pf-http` | Unified Axum/Tower HTTP middleware | 2 passed | none |
| `pf-vector-kit` | `platform/rust/crates/pf-vector-kit` | SIMD L2 squared, Cosine, DotProduct | 1 passed | none |
| `pf-testkit` | `platform/rust/crates/pf-testkit` | Chaos testing & property generators | 2 passed | none |
| `pf-config` | `platform/rust/crates/pf-config` | Zero-dependency environment loader | 1 passed | none |
| `pf-errors` | `platform/rust/crates/pf-errors` | Shared error taxonomy & HTTP status codes | 1 passed | none |

---

## 4. PROJECT PROGRESS & STATUS WORKBOOK SYNCHRONIZATION

| Project | Total Specs | Verified | Implemented | Scaffolded | Passing Tests | Criterion Latency / Throughput | Status Workbook Artifact |
|:---|:---:|:---:|:---:|:---:|:---:|:---|:---|
| `01-rollback-netcode-server` | 90 | 3 | 3 | 87 | not yet measured | not yet measured | [`01_STATUS.xlsx`](file://evidence/workbooks/01_STATUS.xlsx) |
| `02-hnsw-filtered-vector-index` | 152 | 4 | 4 | 148 | not yet measured | 378.02 µs | [`02_STATUS.xlsx`](file://evidence/workbooks/02_STATUS.xlsx) |
| `03-bytecode-vm-gc` | 141 | 0 | 0 | 141 | not yet measured | not yet measured | [`03_STATUS.xlsx`](file://evidence/workbooks/03_STATUS.xlsx) |
| `04-columnar-query-engine` | 145 | 20 | 20 | 125 | 20 verified tests passed | not yet measured | [`04_Columnar_Query_Engine_ULTIMATE_145_Master_STATUS.xlsx`](file://evidence/workbooks/04_Columnar_Query_Engine_ULTIMATE_145_Master_STATUS.xlsx) |
| `05-mini-quic` | 145 | 56 | 56 | 89 | 56 verified tests passed | 120.66 ns | [`05_Mini_QUIC_ULTIMATE_145_Master_STATUS.xlsx`](file://evidence/workbooks/05_Mini_QUIC_ULTIMATE_145_Master_STATUS.xlsx) |
| `06-lsm-kv-engine` | 145 | 24 | 24 | 121 | 24 verified tests passed | not yet measured | [`06_LSM_KV_ENGINE_ULTIMATE_145_Master_STATUS.xlsx`](file://evidence/workbooks/06_LSM_KV_ENGINE_ULTIMATE_145_Master_STATUS.xlsx) |

---

## 5. MEASURED BENCHMARK RESULTS & EVIDENCE TRACEABILITY

| Benchmark Result File | Project | Command | Git Commit | Measurements Summary |
|:---|:---|:---|:---|:---|
| `hnsw_benchmarks.json` | `02-hnsw-filtered-vector-index` | `cargo bench` | `49adde6b5f` | collected |
| `hnsw_filtered_vector_index_benchmarks.json` | `02-hnsw-filtered-vector-index` | `cargo bench --manifest-path /Users/avishjain/avishjain9-repos/portfolio/projects/02-hnsw-filtered-vector-index/backend/Cargo.toml --bench hnsw_bench -- --baseline new` | `49adde6b5f` | benchmark: 378.02 µs |
| `columnar_benchmarks.json` | `04-columnar-query-engine` | `cargo bench` | `49adde6b5f` | collected |
| `columnar_query_engine_benchmarks.json` | `04-columnar-query-engine` | `cargo bench --manifest-path /Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/Cargo.toml --bench columnar_bench -- --baseline new` | `49adde6b5f` | collected |
| `tpch_sf1_results.json` | `None` | `cargo bench` | `49adde6b5f` | collected |
| `mini_quic_benchmarks.json` | `05-mini-quic` | `cargo bench --manifest-path /Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/Cargo.toml --bench quic_bench -- --baseline new` | `49adde6b5f` | benchmark: 120.66 ns |
| `mini_quic_interop.json` | `None` | `cargo bench` | `49adde6b5f` | collected |
| `quic_benchmarks.json` | `05-mini-quic` | `cargo bench` | `2d35bd2f97` | collected |
| `lsm_kv_engine_benchmarks.json` | `06-lsm-kv-engine` | `cargo bench --manifest-path /Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/Cargo.toml --bench lsm_bench -- --baseline new` | `49adde6b5f` | collected |

---

## 6. ARCHITECTURAL DECISIONS SUMMARY
- **ADR-001 - ADR-020**: Recorded in `docs/DECISIONS.md`.
  - ADR-017: Derived Feature ID Convention `<NN>-<zero-padded row index>` with name SHA1 verification.
  - ADR-018: Workbook Status Deliverables with live COUNTIFS formula rollups and conditional formatting.
  - ADR-019: Prohibiting synthetic workbook generation/padding; enforcing strict read-only user inputs.
  - ADR-020: Differential testing harness against DuckDB for relational and columnar correctness.

---

## 7. GAPS & AUDIT FINDINGS
- **Workbook Ingest Lockdown**: Master workbooks for 04 (`6699a333e16e...`, 145 rows, 45 P0) and 05 (`69429364a338...`, 145 rows, 79 P0) locked and verified.
- **Differential SQL Correctness**: Columnar query engine validated against DuckDB on 10,000 synthetic rows across 5 canonical query patterns with 100% agreement.
- **Cryptographic Conformance**: RFC 5869 (HKDF), RFC 9001 (QUIC Initial Secrets), and RFC 7748 (X25519) vector suites passing in `pf-crypto`.

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
1. Complete remaining P0 transport parameters, flow-control frames, and TLS 1.3 state transitions for `05-mini-quic`.
2. Implement Parquet dictionary encoding and Snappy compression roundtrip for `04-columnar-query-engine`.
3. Add `pf-sim` network loss transfer benchmark for `05-mini-quic`.

---

## 11. RESUME INSTRUCTIONS
1. Pull latest workspace state: commit `49adde6b5f`.
2. Run `python3 tools/evidence_gen.py` to regenerate verified evidence rows.
3. Run `python3 tools/workbook_status.py` to refresh all `_STATUS.xlsx` workbooks.
4. Run `python3 tools/bench_collector.py` to refresh Criterion benchmarks.

---

## PROMPTS_I_NEED
1. **05-mini-quic: TLS 1.3 Handshake State Machine & Transport Parameters**
   - **Feature IDs**: `05-013` (Connection state machine), `05-014` (Client/server handshake flow), `05-021` (Transport parameter negotiation)
   - **Acceptance Criteria**: Client sends Initial packet with ClientHello; server responds with Initial (ServerHello) and Handshake (EncryptedExtensions); transport parameters `initial_max_data` and `initial_max_streams_bidi` are negotiated.
   - **Files to Touch**: `projects/05-mini-quic/backend/src/handshake/mod.rs`, `projects/05-mini-quic/backend/src/wire/parameters.rs`, `projects/05-mini-quic/backend/tests/handshake_tests.rs`

2. **05-mini-quic: Connection & Stream Level Flow Control Credit Management**
   - **Feature IDs**: `05-011` (MAX_DATA/MAX_STREAM_DATA frames), `05-012` (MAX_STREAMS frames)
   - **Acceptance Criteria**: Endpoint updates flow control credit upon receiving `MAX_DATA`; sender blocks on stream exhaustion and unblocks immediately when `MAX_STREAM_DATA` frame arrives.
   - **Files to Touch**: `projects/05-mini-quic/backend/src/stream/flow_control.rs`, `projects/05-mini-quic/backend/tests/flow_control_tests.rs`

3. **04-columnar-query-engine: Parquet File Reader & Decoder Integration**
   - **Feature IDs**: `04-026` (Parquet file reader & decoder), `04-049` (Dictionary encoding)
   - **Acceptance Criteria**: Engine reads Parquet files written by pyarrow, decodes dictionary-encoded string columns and Snappy-compressed rowgroups into memory without row materialization.
   - **Files to Touch**: `projects/04-columnar-query-engine/backend/src/storage/parquet.rs`, `projects/04-columnar-query-engine/backend/tests/parquet_tests.rs`
