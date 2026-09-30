# 12 Milestones: Usage-Based Metering & Invoicing Engine

## 1. Specification Overview [Specified]
High-throughput usage metering and proration billing engine treating invoices as pure deterministic functions of events.

## 2. Core Mechanisms [Specified]
Idempotent event ingestion, event-time aggregation windows with late-arriving corrections, tiered/volume/graduated pricing engine, proration, promotional credits, invoice generation as a pure function of (events, plan version), drift reconciliation.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
