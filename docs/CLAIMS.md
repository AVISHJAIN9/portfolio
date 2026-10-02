# Quantitative Claims and Verification Register

Every quantitative metric, timing measurement, or benchmark result cited in the project documentation is registered below, linked to its primary evidence file, and paired with its reproduction command.

| Claim Description | Project | Measured Value | Primary Evidence File | Reproduction Command | Reproduced in S12? |
|:---|:---:|:---:|:---|:---|:---:|
| TPC-H SF1 Q1 Engine Median Execution Time | `04` | `145.68 ms` | [`projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json`](file:///projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json) | `python3 projects/04-columnar-query-engine/tests/tpch/run_tpch.py` | ✓ YES |
| TPC-H SF1 Q6 Engine Median Execution Time | `04` | `21.1 ms` | [`projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json`](file:///projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json) | `python3 projects/04-columnar-query-engine/tests/tpch/run_tpch.py` | ✓ YES |
| TPC-H SF1 Q3 Engine Median Execution Time | `04` | `55.3 ms` | [`projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json`](file:///projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json) | `python3 projects/04-columnar-query-engine/tests/tpch/run_tpch.py` | ✓ YES |
| TPC-H SF1 Lineitem Dataset Row Count | `04` | `6,001,215 rows` | [`projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json`](file:///projects/04-columnar-query-engine/benchmarks/results/tpch_sf1_results.json) | `python3 projects/04-columnar-query-engine/tests/tpch/run_tpch.py` | ✓ YES |
| SQL Differential Testing Total Queries | `04` | `105 distinct queries` | [`evidence/differential_results.json`](file:///evidence/differential_results.json) | `python3 projects/04-columnar-query-engine/tests/differential/run_differential_tests.py` | ✓ YES |
| SQL Differential Testing Match Rate | `04` | `315/315 executions (100%)` | [`evidence/differential_results.json`](file:///evidence/differential_results.json) | `python3 projects/04-columnar-query-engine/tests/differential/run_differential_tests.py` | ✓ YES |
| Mini-QUIC Socket Interop Drop Ratio Window | `05` | `4.81% - 5.23% (3%-7% gate)` | [`evidence/interop_results.json`](file:///evidence/interop_results.json) | `python3 tests/interop/run_interop.py` | ✓ YES |
| Mini-QUIC Interop Negative Controls | `05` | `4/4 passed` | [`evidence/interop_results.json`](file:///evidence/interop_results.json) | `python3 tests/interop/run_interop.py` | ✓ YES |
| LSM KV Engine 1M Ops Write Amplification (without WAL) | `06` | `1.67` | [`evidence/lsm_bench_results.json`](file:///evidence/lsm_bench_results.json) | `python3 projects/06-lsm-kv-engine/tests/bench/run_lsm_bench.py` | ✓ YES |
| LSM KV Engine 1M Ops Write Amplification (with WAL) | `06` | `2.67` | [`evidence/lsm_bench_results.json`](file:///evidence/lsm_bench_results.json) | `python3 projects/06-lsm-kv-engine/tests/bench/run_lsm_bench.py` | ✓ YES |
| LSM KV Engine Read Amplification | `06` | `1.08 (92.1% cache hit)` | [`evidence/lsm_bench_results.json`](file:///evidence/lsm_bench_results.json) | `python3 projects/06-lsm-kv-engine/tests/bench/run_lsm_bench.py` | ✓ YES |
| LSM KV Engine fsync=Always Throughput | `06` | `8,420 ops/sec` | [`evidence/lsm_bench_results.json`](file:///evidence/lsm_bench_results.json) | `python3 projects/06-lsm-kv-engine/tests/bench/run_lsm_bench.py` | ✓ YES |
