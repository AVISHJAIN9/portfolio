# 09 Testing Strategy: Distributed Tracing with Tail Sampling

## 1. Specification Overview [Specified]
OTLP-compliant distributed tracing backend with memory-bounded buffers, tail-based sampling policies, and dependency graphs.

## 2. Core Mechanisms [Specified]
OTLP protobuf ingest, per-trace circular buffering with decision wait window, sampling policies (error, latency, rate, probabilistic), consistent trace-ID hashing across collectors, memory bounds for incomplete traces, span storage & inverted indexes, service DAG.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
