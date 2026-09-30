# Monorepo Capability Reuse & Dependency Map (REUSE_MAP)

This document establishes the capability clustering and reuse topology across all 20 projects in Portfolio Forge. In accordance with Rule R3 (Reuse-First), any capability required by two or more projects is implemented **once** in `platform/` with stable APIs, comprehensive tests, and benchmarks.

---

## 1. Rust Platform Crates (`platform/rust/crates`)

| Platform Crate | Public API & Responsibility | Consuming Projects (>=2) | Owner Workstream | Status |
|---|---|---|---|---|
| **`pf-bytes`** | Zero-copy byte buffers, variable-length integer encoding (varint, QUIC varint), bit reader/writer, endian helpers. | `01`, `04`, `05`, `06`, `08`, `12`, `13`, `14` | Platform Core | Specified |
| **`pf-checksum`** | Hardware-accelerated CRC32C, xxHash64, Adler32 with pure-software fallback paths; Blake3 hashing helpers. | `01`, `04`, `06`, `08`, `09`, `13` | Platform Core | Specified |
| **`pf-collections`** | Lockless ring buffers, SkipList, Bloom / Cuckoo / XOR filters, Slab arena allocator, Radix prefix tree. | `01`, `02`, `03`, `06`, `08`, `09`, `10`, `11`, `15` | Platform Data Structs | Specified |
| **`pf-cache`** | High-hit-rate eviction policies (LRU, CLOCK, ARC, W-TinyLFU, S3-FIFO), sharded concurrent cache, hit-rate simulator. | `04`, `06`, `09`, `11`, `12`, `15`, `17` | Platform Storage | Specified |
| **`pf-sync`** | Lock-free SPSC / MPMC ring queues, Seqlock, epoch reclamation helpers, loom concurrency test harnesses. | `01`, `05`, `07`, `08`, `09`, `15` | Platform Concurrency | Specified |
| **`pf-time`** | Hybrid Logical Clock (HLC), monotonic virtual clock traits for simulation, hierarchical timer wheel. | `01`, `05`, `08`, `10`, `13`, `14`, `16` | Platform Time | Specified |
| **`pf-sim`** | Deterministic discrete-event simulation engine, seeded PRNG, network fault injection (loss, jitter, reorder), disk crash sim. | `01`, `05`, `06`, `07`, `08`, `09`, `18` | Platform Simulation | Specified |
| **`pf-wal`** | Segmented append-only write-ahead log, group commit batcher, fsync policies, checksumming, torn-write recovery. | `06`, `08`, `13`, `19` | Platform Storage | Specified |
| **`pf-storage-kit`** | Page manager, buffer pool manager with CLOCK replacement, direct I/O / mmap backends, block compression traits. | `02`, `04`, `06`, `14` | Platform Storage | Specified |
| **`pf-net`** | Asynchronous UDP / TCP socket abstractions over Tokio, frame pacing hooks, packet capture/replay harness. | `01`, `05`, `08`, `12`, `16`, `17` | Platform Network | Specified |
| **`pf-crypto`** | AEAD encryption wrappers, HKDF key derivation, session resumption token generators (Rustls-backed; no custom primitives). | `01`, `05`, `11`, `13`, `16` | Platform Security | Specified |
| **`pf-ratelimit`** | Token Bucket, Generic Cell Rate Algorithm (GCRA), sliding-window rate limiters with atomic memory & Redis backends. | `01`, `05`, `16`, `17`, `18`, `20` | Platform Traffic | Specified |
| **`pf-hashring`** | Consistent hashing ring (Ketama with vnodes), Rendezvous hashing, Jump Consistent Hash. | `08`, `09`, `11`, `17` | Platform Distributed | Specified |
| **`pf-metrics`** | HDR Histogram latency recorder, atomic counters/gauges, Prometheus scrape exporter, OpenTelemetry tracing glue. | **All 20 Projects** | Platform Observability | Specified |
| **`pf-http`** | Axum REST/WebSocket scaffolding, OpenAPI generation, health/ready probes, problem+json error formatting, SSE helpers. | `01`, `02`, `04`, `07`, `08`, `09`, `10`, `11`, `12`, `13`, `14`, `15`, `16`, `17`, `18`, `19`, `20` | Platform API | Specified |
| **`pf-vector-kit`** | SIMD-accelerated L2 distance, cosine similarity, dot product kernels with runtime AVX2/NEON dispatch & scalar fallback. | `02`, `04` | Platform SIMD | Specified |
| **`pf-testkit`** | Proptest strategies, golden-file regression checkers, fuzzer harness wrappers, criterion regression gate. | **All 20 Projects** | Platform Testing | Specified |
| **`pf-config`** | Hierarchical YAML/ENV configuration loader with validation and secret masking. | **All 20 Projects** | Platform Core | Specified |
| **`pf-errors`** | Structured typed error hierarchy with Problem Details RFC 7807 serialization. | **All 20 Projects** | Platform Core | Specified |

---

## 2. Shared Python ML Package (`platform/python/pf_ml`)

| Submodule | Capabilities | Consumers | Status |
|---|---|---|---|
| **`pf_ml.data`** | Seeded synthetic data generators (timeseries, network traces, query workloads, vector clusters), dataset card templates. | **All 20 Projects** | Specified |
| **`pf_ml.detect`** | Streaming EWMA/CUSUM anomaly detector, Seasonal-Hybrid ESD, Isolation Forest, Population Stability Index (PSI) drift detector. | `01`, `06`, `07`, `09`, `11`, `13`, `14`, `18`, `19` | Specified |
| **`pf_ml.learn`** | Adaptive 1D Kalman filter, monotonic Ridge regression forecaster, lightweight Q-learning / bandit tuner. | `01`, `02`, `04`, `05`, `08`, `12`, `15`, `16`, `20` | Specified |
| **`pf_ml.llm`** | Provider-agnostic LLM interface (Mock, OpenAI, Anthropic, Gemini) with tool calling, JSON schema validation, and offline golden-eval harness. | **All 20 Projects** | Specified |
| **`pf_ml.ops`** | File-based model registry, ONNX export shims, sub-millisecond rollback manager, append-only JSONL experiment tracker. | **All 20 Projects** | Specified |
| **`pf_ml.safety`** | Hard clamping envelopes $[min, max]$, confidence gating thresholds, deterministic default fallbacks, emergency kill-switches. | **All 20 Projects** | Specified |

---

## 3. Shared Web Frontend Packages (`platform/web/packages`)

| Package | Capabilities | Consuming Projects | Status |
|---|---|---|---|
| **`@pf/ui-kit`** | Design tokens, Dark Theme preset, Accessible components (Button, Modal, Toast, Input, Badge, Table), Lucide icon kit. | **All 20 Projects** | Specified |
| **`@pf/scroll3d`** | Progressive WebP frame sequence canvas player with Lenis smooth scroll, GSAP ScrollTrigger scrubbing, and mobile fallback. | **All 20 Projects** | Specified |
| **`@pf/viz`** | Reusable real-time chart components (Latency Histogram, Heatmap, Flamegraph, Queue Depth Stream, DAG Graph). | **All 20 Projects** | Specified |
| **`@pf/realtime-client`**| Resilient WebSocket / SSE client with exponential backoff reconnect, message sequence deduplication, and state sync. | **All 20 Projects** | Specified |

---

## 4. Shared Database & Infrastructure (`platform/db`, `platform/infra`)

| Asset | Capabilities | Consumers | Status |
|---|---|---|---|
| **`platform/db/compose`** | Composable Docker Compose fragments for PostgreSQL 16 (with WAL archiving) and Redis 7.2. | All database-enabled projects | Specified |
| **`platform/db/migrations`**| Versioned, reversible SQL migration runner with schema verification. | `01`, `02`, `04`, `07`, `10`, `11`, `13`, `14`, `18`, `19`, `20` | Specified |
| **`platform/infra/docker-compose.base.yml`** | Base telemetry stack (OpenTelemetry Collector, Prometheus, Grafana, Tempo, MinIO). | **All 20 Projects** | Specified |
| **`platform/infra/github-actions`** | Reusable CI workflows (Rust fmt/clippy/test/audit, Node lint/build, Python ruff/eval, Gitleaks scan). | **All 20 Projects** | Specified |
| **`platform/infra/helm`** | Reusable Helm chart template for Kubernetes microservice deployment. | **All 20 Projects** | Specified |
