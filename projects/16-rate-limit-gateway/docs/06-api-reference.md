# 06 Api Reference: Distributed Rate-Limiting API Gateway

## 1. Specification Overview [Specified]
High-throughput API gateway with Token Bucket, Sliding Window, and GCRA algorithms with Redis Lua and local approximate sync.

## 2. Core Mechanisms [Specified]
Token bucket, sliding window counter, GCRA (Generic Cell Rate Algorithm); Redis Lua atomic counters and local-sync approximate mode with mathematically bounded error; tiered quotas, circuit breaking, zero-downtime hot config reload.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
