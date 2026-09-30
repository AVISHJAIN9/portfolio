# 05 Data Model: Distributed Partitioned PubSub Broker

## 1. Specification Overview [Specified]
Kafka/NATS-style distributed broker with segmented append-only logs, consumer group rebalancing, and Raft-lite replication.

## 2. Core Mechanisms [Specified]
Partitioned append-only log with segments and sparse index, consumer groups and cooperative rebalancing, offset commits, acknowledgment and redelivery state machine, Raft-lite replication, log retention and compaction, backpressure, idempotent producers.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
