# 08 Security And Failure Modes: RFC 9111 Compliant CDN Caching Proxy

## 1. Specification Overview [Specified]
High-performance reverse caching proxy with RFC 9111 HTTP caching semantics, memory/disk tiering, and request coalescing.

## 2. Core Mechanisms [Specified]
Strict RFC 9111 semantics (Cache-Control, ETag, Vary, revalidation), stale-while-revalidate and stale-if-error, single-flight request coalescing (thundering herd protection), memory-then-disk tiering with size-aware eviction, purge by key and surrogate tag, HTTP range requests, negative caching.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
