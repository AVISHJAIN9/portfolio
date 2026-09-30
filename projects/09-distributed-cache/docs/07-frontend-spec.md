# 07 Frontend Spec: Distributed Cache with Pluggable Eviction

## 1. Specification Overview [Specified]
RESP-compatible distributed cache with consistent hashing, gossip membership, ARC/TinyLFU eviction, and single-flight stampede protection.

## 2. Core Mechanisms [Specified]
Consistent hashing with virtual nodes, replication and quorum reads/writes, gossip membership protocol, LRU/LFU/ARC/TinyLFU behind one pluggable interface, hierarchical timing wheel TTL, single-flight coalescing against cache stampedes, hot-key detection, RESP protocol.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
