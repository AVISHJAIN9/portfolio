# 11 Deployment: Oversell-Proof Flash Sale Engine

## 1. Specification Overview [Specified]
High-concurrency inventory reservation platform comparing row locks, optimistic versions, Redis Lua, and sharded counters.

## 2. Core Mechanisms [Specified]
Head-to-head comparison of row locks vs optimistic versioning vs Redis Lua vs sharded counters, TTL-based reservations, idempotent checkout, fair virtual waiting room, hot-key mitigation, cache-database reconciliation worker.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
