# 03 Architecture: LLM Serving Inference Scheduler

## 1. Specification Overview [Specified]
Paged KV-cache memory allocator, continuous batching scheduler, and chunked prefill discrete-event simulation engine.

## 2. Core Mechanisms [Specified]
Continuous batching, paged KV-cache block allocator, chunked prefill, preemption and swapping, token-budget fair queuing, prefix caching, admission control, discrete-event simulation lab with calibrated latency model.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
