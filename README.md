# Systems Engineering Portfolio (High-Performance Rust Monorepo)

A production-grade, rigorously tested monorepo featuring three core systems built from scratch in Rust: a **Vectorized Columnar Query Engine**, an **RFC 9000 Mini-QUIC Transport Engine**, and a **Multi-Level LSM-Tree Storage Engine**.

Every feature claim in this repository is mechanically backed by executable test code, differential testing against reference oracles (DuckDB v1.5.6 and `aioquic` v1.3.0), and reproducible benchmark artifacts.

---

## 1. Core Systems Overview & Verified Feature Status

| Slot | Project Name | Total Specs | Verified P0 | Verified P1 | Total Verified | Architectural Highlights | Primary Verification Artifacts |
|:---:|:---|:---:|:---:|:---:|:---:|:---|:---|
| **04** | [**Columnar Query Engine**](file:///projects/04-columnar-query-engine) | 145 | **42** / 43 | **14** | **56** | Morsel-driven parallelism, 3VL ANSI SQL NULLs, ZoneMap pruning, Parquet scans, disk spilling | [`evidence/differential_results.json`](file:///evidence/differential_results.json), [`tpch_sf1_results.json`](file:///projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json) |
| **05** | [**Mini-QUIC Transport Protocol**](file:///projects/05-mini-quic) | 145 | **75** / 76 | **16** | **91** | Pure-Rust RFC 9000, in-tree TLS 1.3 handshake, CUBIC CC, multi-event ECN, multi-path validation | [`evidence/interop_results.json`](file:///evidence/interop_results.json), [`mini_quic_repro.log`](file:///evidence/repro/mini_quic_repro.log) |
| **06** | [**LSM-Tree KV Storage Engine**](file:///projects/06-lsm-kv-engine) | 145 | **117** / 117 | **16** | **133** | Concurrent MemTable, atomic WAL with group commit, prefix-compressed SSTables, MVCC, ChaCha20 AEAD | [`evidence/lsm_bench_results.json`](file:///evidence/lsm_bench_results.json), [`feature_evidence.csv`](file:///evidence/feature_evidence.csv) |
| **Total** | **Active Core Monorepo** | **435** | **234** / 236 | **46** | **280** | **100% Deterministic Rust Systems** | **Full CI Suite: `make ci`** |

---

## 2. One-Command Live Demos

Each project includes a self-contained, reproducible live demonstration:

### 1. Columnar Query Engine: 105-Query Differential Test vs DuckDB
```bash
# Execute 105 distinct SQL queries across 3 seeds against DuckDB v1.5.6 with 3VL NULL verification
python3 projects/04-columnar-query-engine/tests/differential/run_differential_tests.py
```

### 2. Mini-QUIC: Real Socket Interop with `aioquic` over Lossy Proxy
```bash
# Run authentic client/server UDP socket interop with aioquic transferring 1 MiB pseudo-random payloads
python3 tests/interop/run_interop.py
```

### 3. LSM KV Engine: 1M Operations Multi-Level Benchmark
```bash
# Execute 1,000,000 operations across 3 LSM levels measuring Write & Read Amplification
python3 projects/06-lsm-kv-engine/tests/bench/run_lsm_bench.py
```

---

## 3. Verified Performance & Quantitative Benchmarks

| Metric | Project | Measured Outcome | Reference Oracle / Baseline | Evidence File |
|:---|:---:|:---:|:---:|:---|
| **TPC-H SF1 Q1** | `04` | **145.68 ms** | DuckDB: 151.75 ms | [`tpch_sf1_results.json`](file:///projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json) |
| **TPC-H SF1 Q6** | `04` | **21.10 ms** | DuckDB: 22.94 ms | [`tpch_sf1_results.json`](file:///projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json) |
| **SQL Differential Equivalence** | `04` | **100% Match (315/315)** | DuckDB v1.5.6 (3 seeds, 100k rows) | [`differential_results.json`](file:///evidence/differential_results.json) |
| **QUIC Socket Interop Drop Ratio** | `05` | **4.81% - 5.23%** | Configured 5.0% drop proxy | [`interop_results.json`](file:///evidence/interop_results.json) |
| **QUIC Security & Negative Controls** | `05` | **4/4 Passed** | ALPN mismatch, bitflip AEAD, timeout | [`interop_results.json`](file:///evidence/interop_results.json) |
| **LSM Write Amplification (No WAL)** | `06` | **1.67** | Multi-level leveled compaction | [`lsm_bench_results.json`](file:///evidence/lsm_bench_results.json) |
| **LSM Write Amplification (With WAL)**| `06` | **2.67** | Sequential append + compaction | [`lsm_bench_results.json`](file:///evidence/lsm_bench_results.json) |
| **LSM Read Amplification** | `06` | **1.08** | 92.1% Block Cache Hit Ratio | [`lsm_bench_results.json`](file:///evidence/lsm_bench_results.json) |
| **LSM fsync=Always Throughput** | `06` | **8,420 ops/sec** | Synchronous WAL fsync | [`lsm_bench_results.json`](file:///evidence/lsm_bench_results.json) |

---

## 4. Deep-Dive Documentation & System Design

- [**Quantitative Claims & Verification Register (`docs/CLAIMS.md`)**](file:///docs/CLAIMS.md): Exact reproduction commands for every claimed number.
- [**Architectural Decision Records (`docs/DECISIONS.md`)**](file:///docs/DECISIONS.md): 28 ADRs detailing technology choices, trade-offs, and invariants.
- [**Project 04 Interview Notes (`projects/04-columnar-query-engine/INTERVIEW_NOTES.md`)**](file:///projects/04-columnar-query-engine/INTERVIEW_NOTES.md): 60s pitch, bug post-mortems, and 15 grounded technical Q&As.
- [**Project 05 Interview Notes (`projects/05-mini-quic/INTERVIEW_NOTES.md`)**](file:///projects/05-mini-quic/INTERVIEW_NOTES.md): Wire protocols, loss recovery dynamics, and interop architecture.
- [**Project 06 Interview Notes (`projects/06-lsm-kv-engine/INTERVIEW_NOTES.md`)**](file:///projects/06-lsm-kv-engine/INTERVIEW_NOTES.md): Crash recovery, SSTable compression, and compaction mechanics.

---

## 5. Building and Running Tests Locally

```bash
# Run all unit and integration test suites
make test

# Run full portfolio linting, tests, security audits, and workbook sync
make ci

# Build reproducible release binaries
make repro
```
