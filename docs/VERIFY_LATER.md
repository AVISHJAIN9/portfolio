# Deferred Verification Tasks (docs/VERIFY_LATER.md)

This document tracks verification tasks, mutation testing, fuzz runs, line coverage checks, and weak-test audits deferred from Session 10 for execution in a dedicated verification session.

---

## 1. Unvalidated Partial Mutation Data (Session 10)
The following partial cargo-mutants data was observed prior to override. It is recorded here as unvalidated partial data and is not used as a gating requirement in this session:

| Module | Viable Mutants | Caught | Missed | Timeout | Unviable | Caught % | Status |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| `06_wal` (`src/wal/mod.rs`) | 43 | 39 | 4 | 0 | 3 | 90.7% | Unvalidated Partial |
| `06_manifest` (`src/manifest/mod.rs`) | 12 | 11 | 1 | 0 | 1 | 91.7% | Unvalidated Partial |
| `06_compaction` (`src/compaction/leveled.rs`) | 48 | 41 | 7 | 0 | 3 | 85.4% | Unvalidated Partial |
| `04_morsel` (`src/execution/morsel.rs`) | 25 | 24 | 0 | 1 | 6 | 96.0% | Unvalidated Partial |

---

## 2. Deferred Mutation Testing Runs (`cargo-mutants`)
To be scheduled and run in dedicated verification session:
- **Project 06 (LSM KV Engine)**:
  - `backend/src/wal/mod.rs`
  - `backend/src/manifest/mod.rs`
  - `backend/src/compaction/leveled.rs`
- **Project 04 (Columnar Query Engine)**:
  - `backend/src/execution/morsel.rs`
  - `backend/src/optimizer/sql.rs` (lexer/parser)
- **Project 05 (Mini-QUIC)**:
  - `backend/src/wire/codec.rs` & `src/wire/packet.rs`
  - `backend/src/wire/frame.rs`
  - `backend/src/recovery/mod.rs`

---

## 3. Deferred Fuzzing Targets (`cargo-fuzz`, 120s each)
- **Project 04**: `sql_parse`
- **Project 05**: `packet_decode`, `frame_decode`, `tls_handshake_msg_decode`
- **Project 06**: `wal_replay`, `sstable_open`, `manifest_replay`

---

## 4. Deferred Coverage Audits (`cargo-llvm-cov`)
- **Project 05**: Line coverage check >= 85% across `src/wire/` (`packet`, `codec`, `frame`).

---

## 5. Weak-Test Review Items (Verified-with-Caveat)
- **05-001, 05-002, 05-003**: Packet header decoding and encoding. Verified in unit tests; vectors to be cross-checked against RFC 9001 Appendix A.2 sample bytes.
- **06-017 vs 06-021**: WAL checksum calculation vs corruption detection and torn tail recovery. Verified in `wal_tests.rs`.
- **06-018**: WAL fsync policy modes. Durability modes verified; inject formal mock sync sink counter in future test iteration.
- **06-056**: Compaction scheduling. Thresholds, score ordering, and busy guards verified across multi-level tests.
- **04-040, 04-041**: Predicate pushdown and projection pruning. Verified with plan-shape and zone-map execution assertions.
- **04-079**: Selection vectors. Verified; unified `SelectionVector` structure used across vectorized filter and morsel parallel paths.
