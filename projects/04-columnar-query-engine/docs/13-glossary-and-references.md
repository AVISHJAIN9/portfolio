# 13 Glossary And References: Columnar Query Engine (DuckDB Style)

## 1. Specification Overview [Specified]
Vectorized columnar SQL analytics engine with Parquet-like format, Morsel-driven parallelism, and cost-based optimizer.

## 2. Core Mechanisms [Specified]
Columnar format with row groups, dictionary/RLE/delta encodings, zone maps, SQL parser to logical to physical plan, vectorized operators (1024-row batches, selection vectors), hash join & hash aggregate, filter pushdown, cost-based join ordering, morsel-driven parallelism.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
