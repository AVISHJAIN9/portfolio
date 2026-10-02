# Project 04: Columnar Query Engine — Interview Notes & System Design

## 1. 60-Second Pitch
The **Columnar Query Engine** is a vectorized, morsel-driven analytical query execution engine written in Rust. It compiles declarative SQL statements into logical and physical execution DAGs, reading columnar Parquet datasets and partitioned chunk vectors. Using SIMD-friendly `SelectionVector` operators, cost-based predicate pushdown, min/max chunk-level ZoneMap pruning, S3-FIFO cache replacement, and zero-copy buffer pooling, it delivers low-latency analytical query performance. Across 105 distinct differential queries and TPC-H SF1 benchmarks against DuckDB v1.5.6, it achieves exact result set parity with 100% equivalence on complex 3VL NULL semantics, multi-table hash joins, and grouped aggregations.

---

## 2. Architecture Walk-Through
The engine processes queries through a layered columnar pipeline:

```
[SQL Text] 
   │
   ▼ [projects/04-columnar-query-engine/backend/src/optimizer/sql.rs]
[SqlParser & AST Generation]
   │
   ▼ [projects/04-columnar-query-engine/backend/src/optimizer/cost_model.rs]
[Logical & Cost-Based Optimizer (Predicate Pushdown, Projection Pruning)]
   │
   ▼ [projects/04-columnar-query-engine/backend/src/execution/morsel.rs]
[Morsel Scheduler & Thread Pool (Lock-Free Work Stealing)]
   │
   ▼ [projects/04-columnar-query-engine/backend/src/storage/parquet_reader.rs & zonemap.rs]
[ZoneMap RowGroup Pruning & Vectorized Storage Scans]
   │
   ▼ [projects/04-columnar-query-engine/backend/src/execution/vector.rs]
[Vectorized Filter & SelectionVector Evaluation]
   │
   ▼ [projects/04-columnar-query-engine/backend/src/execution/join.rs & aggregate.rs]
[Radix Hash Joins & SIMD Aggregations]
   │
   ▼ [projects/04-columnar-query-engine/backend/src/storage/buffer_pool.rs & memory_pool.rs]
[Zero-Copy Buffer Pool & Memory Budget Enforcement]
```

### Key Source Components:
- **Parser & AST**: [`src/optimizer/sql.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/optimizer/sql.rs) parses SQL tokens into structured AST nodes.
- **Optimizer & Cost Model**: [`src/optimizer/cost_model.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/optimizer/cost_model.rs) ranks predicate selectivity (equality > range) and prunes unused projections.
- **Morsel Execution**: [`src/execution/morsel.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/morsel.rs) divides data into 100k-row morsels dynamically scheduled across worker threads.
- **Vectorized Operators**: [`src/execution/vector.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/vector.rs) and [`src/execution/aggregate.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/aggregate.rs) execute columnar evaluations without row materialization.
- **Storage & Parquet**: [`src/storage/parquet_reader.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/parquet_reader.rs) and [`src/storage/zonemap.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/zonemap.rs) load horizontal row groups and prune non-matching chunks using min/max stats.
- **Memory & Spilling**: [`src/storage/spill.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/spill.rs) and [`src/storage/memory_pool.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/memory_pool.rs) enforce strict RAM budgets and spill intermediate batches to temporary disk files.

---

## 3. The 5 Hardest Bugs Actually Found & Fixed
1. **Three-Valued Logic NULL Filtering in WHERE Clauses**
   - *Symptom*: Differential tests showed 12 queries returning rows where expressions evaluated to `NULL` (UNKNOWN), whereas SQL standard requires dropping non-TRUE rows.
   - *Fix*: Refactored `VectorizedExecutor::evaluate_filter` in `ee82d56` to strictly retain indices evaluating to `true`, matching DuckDB 3VL semantics.
2. **ZoneMap Pruning Miss on Boundary Queries**
   - *Symptom*: Predicates on exact boundary conditions (`val >= max`) were erroneously pruned when `<` was used instead of `<=` in `ZoneMap::can_prune`.
   - *Fix*: Standardized half-open interval checking in `7ecf31d` and added comprehensive edge-case tests in `zonemap_pruning_tests.rs`.
3. **Morsel Work-Stealing Starvation Under Uneven Partitions**
   - *Symptom*: Queries with heavily skewed filter selectivity stalled worker threads while one thread processed the remainder.
   - *Fix*: Implemented dynamic atomic queue popping in `morsel.rs` with lock-free atomic index reservation (`fetch_add`).
4. **SqlParser Lexer Bounds Truncation in Compound Expressions**
   - *Symptom*: Complex queries containing `BETWEEN x AND y` combined with `ORDER BY` caused parser index out-of-bounds panics on multi-predicate WHERE clauses.
   - *Fix*: Added lookahead bounds verification and proper token state advances in `src/optimizer/sql.rs`.
5. **Memory Pool Exceedance Panic on Large Radix Hash Joins**
   - *Symptom*: Building large hash join tables under low memory budget caused process abort rather than returning typed `Err(OutOfMemory)`.
   - *Fix*: Implemented reservation-before-allocation checking in `memory_pool.rs` and integrated disk spilling in `src/storage/spill.rs`.

---

## 4. 8 Key Architectural Decisions & Trade-Offs (from `docs/DECISIONS.md`)
1. **Rust for Engine Implementation (ADR-002)**: Eliminates garbage collection pauses and provides native SIMD intrinsic access for columnar scanning.
2. **Morsel-Driven Parallelism**: Replaced traditional Volcano iteration with morsel-based parallel execution, achieving near-linear multi-core scaling.
3. **Min/Max ZoneMap Chunk Pruning (ADR-013)**: Avoids reading row groups from disk/memory if the query predicate is disjoint with the chunk's `[min, max]` envelope.
4. **SelectionVector In-Place Filtering (ADR-013)**: Operates on index slices instead of allocating new chunk vectors on each filter stage, reducing memory bandwidth by ~4x.
5. **S3-FIFO Result Cache (ADR-014)**: Uses small, main, and ghost queues to prevent large table scans from evicting hot aggregated result sets.
6. **Parquet Integration via Rust `parquet` Crate (ADR-022)**: Reads industry-standard columnar formats directly into chunk representations.
7. **Strict ANSI 3VL NULL Semantics (ADR-028)**: Ensures exact behavioral compatibility with DuckDB and PostgreSQL for NULL comparisons and aggregations.
8. **Dynamic Disk Spilling for Hash Joins & Sorts (04-063, 04-064)**: Partitions out-of-core datasets into sorted temporary runs with atomic file commit guarantees.

---

## 5. System Limitations & Scope Boundaries
- **Supported Scope**: Vectorized columnar queries, morsel scheduling, cost-based predicate pushdown, projection pruning, Parquet scans, hash joins, grouped aggregations, CSV parsing, delta/bit-packing, Bloom filters, disk spill, atomic file commits.
- **Descoped / Excluded**: Frontend/UI, distributed cluster execution, PostgreSQL/MySQL wire protocol emulation, cloud blob connectors (S3/GCS remote streaming), distributed transaction coordination.
- **Manual Verification Dependency**: `04-133` (GitHub Actions CI matrix) requires remote repository runner tokens.

---

## 6. 15 Likely Interview Questions & Code-Grounded Answers

1. **How does the engine handle vectorized execution compared to Volcano iterator models?**
   - *Answer*: Rather than calling `next()` per row, our `VectorizedExecutor` in [`src/execution/vector.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/vector.rs) evaluates contiguous memory slices of 100k values per morsel using tight CPU-pipelined loops, avoiding virtual function dispatch overhead.

2. **How do you prevent memory copies during multiple WHERE filter steps?**
   - *Answer*: We use `SelectionVector` (see [`src/execution/vector.rs:14`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/vector.rs#L14)), which maintains an array of matched row indices. Subsequent filters narrow the `indices` buffer without copying column data.

3. **How does ZoneMap pruning work?**
   - *Answer*: Each horizontal chunk stores `min` and `max` values in `ZoneMap` ([`src/storage/zonemap.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/zonemap.rs)). If a predicate like `val > 500` is evaluated against a chunk with `max = 400`, the entire chunk is skipped before decompression.

4. **How are NULL values represented and processed in 3VL?**
   - *Answer*: In [`src/execution/null_bitmap.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/null_bitmap.rs), we maintain a bitmap where bit `0` indicates NULL. Filters evaluate to boolean states (`TRUE`, `FALSE`, `UNKNOWN`), dropping all rows where the condition is not strictly `TRUE`.

5. **How does your Morsel Scheduler balance uneven workloads across cores?**
   - *Answer*: [`src/execution/morsel.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/morsel.rs) divides data into discrete morsels. Worker threads execute lock-free atomic work-stealing from a global task queue, preventing thread starvation.

6. **How do you optimize hash join execution?**
   - *Answer*: [`src/execution/join.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/join.rs) builds a compact hash table on the smaller build relation using FxHash and probes it using vectorized batch lookups from the probe relation.

7. **How does the cost model choose predicate execution order?**
   - *Answer*: In [`src/optimizer/cost_model.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/optimizer/cost_model.rs), equality filters are ranked with selectivity rank 0, narrow range filters with rank 1, and wide scans with rank 2, minimizing the intermediate working set.

8. **How does the engine handle memory exhaustion during large sorts or aggregations?**
   - *Answer*: The `MemoryPool` tracks allocations against a configured ceiling. When exceeded, `ExternalSorter` ([`src/storage/spill.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/spill.rs)) sorts in-memory runs, flushes them to disk with atomic commits, and performs an external k-way merge.

9. **What encoding techniques are implemented for columnar storage?**
   - *Answer*: In [`src/storage/encoding.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/encoding.rs), we implement Delta Encoding (storing consecutive deltas for sorted integers) and Bit Packing (packing integers into minimal bit widths: 1, 2, 4, 8, 16 bits).

10. **How do Bloom filters accelerate query execution?**
    - *Answer*: In [`src/storage/encoding.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/encoding.rs), Bloom filters with double hashing (`Murmur3` + `FxHash`) allow early rejection of non-existent keys before scanning storage chunks.

11. **How do you verify differential correctness against DuckDB?**
    - *Answer*: `tests/differential/run_differential_tests.py` executes 105 queries across 3 seeds on a 100k-row NULL-heavy dataset against both engines, comparing result schemas, row order, and floating-point tolerances.

12. **How does the engine guarantee crash-safe file commits?**
    - *Answer*: In [`src/storage/spill.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/spill.rs), files are written to `.tmp` files, `fsync`'d, renamed atomically to the target filename, and the parent directory is `fsync`'d.

13. **How does CSV ingestion handle embedded newlines and quotes?**
    - *Answer*: [`src/storage/csv_reader.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/csv_reader.rs) conforms to RFC 4180, maintaining an inside-quotes state machine that preserves embedded newlines and escaped `""` characters.

14. **How does the Buffer Manager handle page pinning?**
    - *Answer*: [`src/storage/buffer_pool.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/storage/buffer_pool.rs) tracks pin counts per frame. Only frames with `pin_count == 0` are candidates for LRU/S3-FIFO eviction.

15. **How are query cancellation and timeouts implemented?**
    - *Answer*: In [`src/execution/morsel.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/04-columnar-query-engine/backend/src/execution/morsel.rs), workers check an `AtomicBool` cancellation token between morsel boundaries and compare against a monotonic instant deadline.

---

## 7. What I Would Build Next
1. **Window Function Pipeline (`04-032`)**: Native support for `ROW_NUMBER()`, `RANK()`, and `SUM() OVER (PARTITION BY ... ORDER BY ...)` in the vectorized execution DAG.
2. **Dynamic JIT Compilation**: Compiling filter expressions into native LLVM/Cranelift machine code to eliminate loop branching overhead.
3. **Cloud Object Storage Streaming**: Direct range-request streaming of remote Parquet files from Amazon S3 / Google Cloud Storage with predictive byte-range prefetching.
