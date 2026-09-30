# 02 Requirements: Real-Time CRDT Rich-Text Engine

## 1. Specification Overview [Specified]
Collaborative rich-text editing engine with sequence CRDT (Fugue/RGA), Peritext formatting spans, and causal op log sync.

## 2. Core Mechanisms [Specified]
Sequence CRDT with unique Lamport operation IDs, tombstones and garbage collection, Peritext formatting spans, causal delivery queue, state-vector delta synchronization, presence awareness, snapshots + op log persistence, offline edit merging.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
