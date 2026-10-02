# Session 12 Semantic Weak-Test Audit Report

Audit conducted across all items from `docs/VERIFY_LATER.md` Section 5.

---

| Feature ID | Workbook Feature Name | Test Function | Test File:Line | Acceptance Criteria & Audit Finding | Verdict |
|:---|:---|:---|:---|:---|:---:|
| `05-001` | QUIC packet header encoding/decoding | `test_quic_packet_header_codec` | `projects/05-mini-quic/backend/tests/packet_codec_tests.rs:9` | Verified against RFC 9000 wire format and RFC 9001 Appendix A.2 test vectors. | **KEEP (STRENGTHENED)** |
| `05-002` | Long-header packet support | `test_long_header_packet_support` | `projects/05-mini-quic/backend/tests/packet_codec_tests.rs:57` | Covers Initial, Handshake, 0-RTT, and Retry header types with distinct token decoding. | **KEEP (STRENGTHENED)** |
| `05-003` | Short-header packet support | `test_short_header_packet_support` | `projects/05-mini-quic/backend/tests/packet_codec_tests.rs:105` | Validates 1-RTT spin bit, key phase bit, and variable-length DCID decoding. | **KEEP (STRENGTHENED)** |
| `06-017` | WAL checksums | `test_wal_checksums_validation` | `projects/06-lsm-kv-engine/backend/tests/wal_tests.rs:45` | Computes CRC32 checksum per record; distinct from corruption recovery. | **KEEP** |
| `06-021` | WAL corruption detection | `test_wal_corruption_detection_crc` | `projects/06-lsm-kv-engine/backend/tests/wal_tests.rs:115` | Detects torn tail and corrupted checksum bytes on startup replay. | **KEEP** |
| `06-018` | fsync policy modes | `test_wal_fsync_policy_modes_durability` | `projects/06-lsm-kv-engine/backend/tests/wal_tests.rs:72` | Asserts mock sync sink call count per durability policy (`Always`, `Batch`, `Periodic`, `NoFsync`). | **KEEP (STRENGTHENED)** |
| `06-056` | Leveled compaction | `test_leveled_compaction_scheduling_triggers` | `projects/06-lsm-kv-engine/backend/tests/compaction_tests.rs:1` | Separate assertions for L0 file threshold, level size ratio multiplier, and non-overlapping locks. | **KEEP** |
| `04-040` | Predicate pushdown | `test_predicate_pushdown_optimization` | `projects/04-columnar-query-engine/backend/tests/sql_tests.rs:248` | Physical plan AST verified to contain pushdown filters directly inside scan operator. | **KEEP** |
| `04-041` | Projection pruning | `test_projection_pruning_optimization` | `projects/04-columnar-query-engine/backend/tests/sql_tests.rs:270` | Physical plan AST verified to omit unreferenced columns prior to morsel execution. | **KEEP** |
| `04-079` | Selection vectors | `test_morsel_selection_vectors` | `projects/04-columnar-query-engine/backend/tests/morsel_tests.rs:25` | Shared `SelectionVector` struct in `src/execution/vector.rs:24` verified across morsels. | **KEEP** |
| `04-078` | Vectorized execution | `test_vectorized_sum_filter_aggregation` | `projects/04-columnar-query-engine/backend/tests/vectorized_exec_tests.rs:10` | Uses unified `SelectionVector` in batch execution loop (`src/execution/vector.rs:24`). | **KEEP** |
