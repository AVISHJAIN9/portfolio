# Project 06: LSM KV Engine — Interview Notes & System Design

## 1. 60-Second Pitch
**LSM KV Engine** is a high-throughput, crash-resilient Log-Structured Merge-Tree storage engine written in Rust. It implements concurrent skiplist memtables, atomic write-ahead logging (WAL) with group-commit and CRC32 verification, prefix-compressed SSTable files with sparse indexes and Bloom filters, leveled and size-tiered compaction, MVCC snapshot isolation, and ChaCha20-Poly1305 authenticated at-rest encryption. Evaluated on a 1,000,000-operation multi-level benchmark across 3 formed levels (L0, L1, L2), it achieves a measured Write Amplification of **1.67** (without WAL) / **2.67** (with WAL) and a Read Amplification of **1.08** with 92.1% block cache hit ratio, while guaranteeing zero data loss across simulated power-loss crashes.

---

## 2. Architecture Walk-Through
The storage engine coordinates mutations and point/range lookups through an integrated multi-tier hierarchy:

```
[Write API: Put / Delete / WriteBatch]
   │
   ├──▶ [projects/06-lsm-kv-engine/backend/src/wal/mod.rs]
   │    [Sequential Append WAL (Group Commit, CRC32, Fsync Modes)]
   │
   └──▶ [projects/06-lsm-kv-engine/backend/src/mem/mod.rs]
        [Active SkipList MemTable (Arena Bump Allocator, Memory Tracker)]
            │
            ▼ (MemTable Size >= 1 MiB)
        [Immutable MemTable]
            │
            ▼ [projects/06-lsm-kv-engine/backend/src/compaction/flush.rs]
        [Background Flush Task] ──▶ [L0 SSTables]
                                         │
                                         ▼ [projects/06-lsm-kv-engine/backend/src/compaction/leveled.rs]
                                    [Multi-Level Leveled Compaction]
                                         │
                                         ├──▶ [L1 Non-Overlapping SSTables]
                                         └──▶ [L2 SSTables]

[Read API: Get / Scan / Snapshot]
   │
   ▼ [projects/06-lsm-kv-engine/backend/src/engine/mod.rs]
[Active MemTable ➔ Immutable MemTable ➔ BlockCache ➔ SSTables (Bloom Filter Check)]
   │
   ▼ [projects/06-lsm-kv-engine/backend/src/cache/mod.rs]
[Sharded LRU Block Cache (Admission/Eviction Policy, Open-File Cache)]
```

### Key Source Components:
- **Unified Engine & MVCC**: [`src/engine/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/engine/mod.rs) coordinates sequence numbers, snapshot isolation, point reads, and range iterators.
- **Write-Ahead Log (WAL)**: [`src/wal/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/wal/mod.rs) serializes mutations with per-record CRC32 checksums, segment rotation, and group commit.
- **MemTable & Arena**: [`src/mem/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/mem/mod.rs) implements concurrent SkipList indexing with bump-allocated arena buffers and memory quota accounting.
- **SSTable Formats & Encryption**: [`src/sstable/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/sstable/mod.rs) writes sorted data blocks with prefix compression, restart points, Bloom filters, and ChaCha20-Poly1305 AEAD block encryption.
- **Compaction & Manifest**: [`src/compaction/leveled.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/compaction/leveled.rs) and [`src/manifest/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/manifest/mod.rs) execute leveled compaction and persist version edits with atomic crash recovery.
- **Block Cache & Open Files**: [`src/cache/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/cache/mod.rs) provides sharded LRU block caching, admission filtering, open file descriptor LRU pooling, and latency histograms.

---

## 3. The 5 Hardest Bugs Actually Found & Fixed
1. **Compaction Manifest Edit Loss on Dirty Power Cut**
   - *Symptom*: Power-loss simulation (`FaultFs`) revealed that after compaction deleted L0 files and created an L1 file, a crash before manifest directory fsync caused startup recovery to read stale level state, causing "missing SSTable" errors.
   - *Fix*: Implemented atomic manifest version logging with parent directory `fsync` and version number monotonicity checks in `c955892`.
2. **MVCC Iterator Phantom Read from Unflushed Tombstone**
   - *Symptom*: Range scan over multiple SSTables resurrected a deleted key because an older L1 SSTable entry was yielded before an L0 tombstone was resolved.
   - *Fix*: Built a unified `MergingIterator` with priority queue ordering by `(Key ASC, SequenceNumber DESC)` in `822b841`.
3. **Prefix Compression Restart Point Boundary Deserialization**
   - *Symptom*: Reading keys across restart intervals corrupted key prefixes when key sharing was attempted across restart boundary boundaries.
   - *Fix*: Enforced full key storage at every $N$-th restart index ($N=16$) in `d298229` and validated with fuzz mutations.
4. **Write Backpressure MemTable Memory Tracker Leak**
   - *Symptom*: Dropping an immutable memtable after background flush failed to decrement the global memory tracker counter, causing permanent write throttling stalls.
   - *Fix*: Encapsulated memory accounting in RAII `ArenaTracker` guards in [`src/mem/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/mem/mod.rs).
5. **Open File Descriptor Exhaustion on Deep SSTable Hierarchy**
   - *Symptom*: Large scans across 500 SSTables exceeded OS file descriptor limits (`EMFILE`).
   - *Fix*: Built `OpenFileCache` in [`src/cache/mod.rs:240`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/cache/mod.rs#L240), capping simultaneous open descriptors to configured limits with LRU handle eviction.

---

## 4. 8 Key Architectural Decisions & Trade-Offs (from `docs/DECISIONS.md`)
1. **Rust for Deterministic Systems Programming (ADR-002)**: Eliminates GC pauses and provides RAII resource cleanup for file descriptors and memory buffers.
2. **LSM Multi-Level Storage Topology (ADR-023)**: Minimizes write amplification while organizing data into leveled tiers ($L_0 \dots L_2$) with exponential capacity scaling.
3. **Atomic WAL with CRC32 Verification (ADR-023)**: Ensures ACID durability across crashes with fast sequential disk writes.
4. **Prefix Compression with Restart Points (06-045)**: Compresses sorted keys by sharing common prefixes, reducing SSTable index size by ~60%.
5. **ChaCha20-Poly1305 At-Rest AEAD Encryption (06-024, 06-127)**: Transparently authenticates and encrypts WAL records and SSTable data blocks without leaking plaintext on disk.
6. **Sharded LRU Block Cache & Admission Control (06-092, 06-093)**: Prevents large scans from evicting hot point-lookup data via 8-shard LRU tables and compaction bypass policies.
7. **Arena Memory Allocation (06-097)**: Allocates memtable node data using continuous bump arenas, releasing all memory simultaneously on memtable drop.
8. **FaultFs Power-Loss Crash Testing (06-123)**: Simulates hardware power failure by dropping un-fsynced dirty kernel buffers, validating zero corruption on recovery.

---

## 5. System Limitations & Scope Boundaries
- **Supported Scope**: Full LSM engine, WAL replay, concurrent SkipList, SSTable format, prefix compression, Bloom filters, leveled/size-tiered compaction, MVCC snapshots, optimistic transactions, AEAD encryption, block cache, open-file cache, resource quotas, latency histograms.
- **Descoped / Excluded**: Distributed Raft consensus replication, network RPC servers (gRPC/Redis wire protocol), PostgreSQL foreign data wrapper, cloud S3 cold storage tiering.
- **Manual Verification Dependency**: None. All core engine and P1 features are fully verified in-tree.

---

## 6. 15 Likely Interview Questions & Code-Grounded Answers

1. **How does an LSM-Tree achieve high write throughput compared to a B-Tree?**
   - *Answer*: B-Trees require random disk I/O to update leaf pages in-place. In our LSM engine, writes append sequentially to the WAL ([`src/wal/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/wal/mod.rs)) and insert into an in-memory SkipList ([`src/mem/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/mem/mod.rs)), converting random writes into sequential disk operations.

2. **How do you resolve point lookups across multiple levels?**
   - *Answer*: In [`src/engine/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/engine/mod.rs), lookups search in chronological order: (1) Active MemTable, (2) Immutable MemTables, (3) L0 SSTables (newest to oldest), (4) L1..Ln SSTables via binary search in the sparse index after checking the Bloom filter.

3. **How does the Bloom filter prevent unnecessary disk reads?**
   - *Answer*: In [`src/sstable/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/sstable/mod.rs), each SSTable stores a Bloom filter built with 10 bits/key (~1% false positive rate). If the filter returns false, we skip loading the SSTable index and data blocks entirely.

4. **How is Write Amplification (WA) calculated and controlled?**
   - *Answer*: Write Amplification is $\frac{\text{Total Disk Bytes Written}}{\text{User Bytes Written}}$. In our benchmark ([`evidence/lsm_bench_results.json`](file:///Users/avishjain/avishjain9-repos/portfolio/evidence/lsm_bench_results.json)), WA is **1.67** without WAL and **2.67** with WAL. Leveled compaction controls WA by selecting files with minimal overlap into the next level.

5. **How does Leveled Compaction work in your engine?**
   - *Answer*: In [`src/compaction/leveled.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/compaction/leveled.rs), Level $L$ has a byte limit ($10^L \times \text{Base}$). When exceeded, a file from $L$ is merged with all overlapping files in $L+1$, writing new non-overlapping SSTables in $L+1$ and updating the manifest atomically.

6. **How do you implement MVCC and snapshot isolation?**
   - *Answer*: Every mutation receives a monotonic sequence number. In [`src/engine/txn.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/engine/txn.rs), a snapshot captures the current sequence number $S_{snap}$. Reads only see keys with sequence numbers $S_{key} \le S_{snap}$, providing consistent point-in-time views without locking writers.

7. **How does the engine handle crash recovery after a sudden power loss?**
   - *Answer*: On startup, [`src/manifest/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/manifest/mod.rs) reconstructs the SSTable level hierarchy. Then [`src/wal/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/wal/mod.rs) replays WAL records from the last flush checkpoint, verifying CRC32 checksums and discarding uncommitted tail torn writes.

8. **How does Prefix Compression reduce SSTable size?**
   - *Answer*: In [`src/sstable/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/sstable/mod.rs), consecutive keys share identical prefixes (e.g. `user:1001`, `user:1002`). We store `(shared_len, unshared_len, value_len, unshared_key, value)`. Every 16th key is stored in full at a restart point for fast binary searching.

9. **How does the Block Cache prevent scan thrashing?**
   - *Answer*: In [`src/cache/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/cache/mod.rs), we implement an admission policy where one-off large sequential table scans and compaction reads bypass the cache, preserving the hot working set in the 8-shard LRU table.

10. **How are file descriptors managed to prevent OS limit exhaustion?**
    - *Answer*: In [`src/cache/mod.rs:240`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/cache/mod.rs#L240), `OpenFileCache` manages open SSTable file handles up to a configured limit, closing LRU handles when the threshold is reached.

11. **How does the Arena Allocator optimize MemTable memory?**
    - *Answer*: [`src/mem/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/mem/mod.rs) uses a bump-allocation arena that eliminates per-node heap allocations. When the memtable is flushed to disk, the entire arena chunk is dropped at once, eliminating heap fragmentation.

12. **How does optimistic concurrency control detect transaction conflicts?**
    - *Answer*: In [`src/engine/txn.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/engine/txn.rs), transactions buffer writes locally. At commit time, the engine checks whether any key in the transaction's read set was modified after the transaction's read sequence number. If so, it aborts with `Err(Conflict)`.

13. **How is data encrypted at rest?**
    - *Answer*: In [`src/security/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/security/mod.rs), WAL records and SSTable data blocks are encrypted with ChaCha20-Poly1305 AEAD. A unique 12-byte nonce per block prevents ciphertext manipulation and ensures authenticated integrity.

14. **How do you monitor latency distribution?**
    - *Answer*: In [`src/cache/mod.rs:330`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/06-lsm-kv-engine/backend/src/cache/mod.rs#L330), `LatencyHistogram` records operations into logarithmic buckets, calculating p50, p95, and p99 metrics with zero floating-point arithmetic on the hot path.

15. **How is write stall and backpressure enforced?**
    - *Answer*: When the immutable memtable count exceeds 2 or L0 SSTable count exceeds 4, the engine sleeps writers incrementally (1ms..10ms) to allow background flush and compaction to catch up, preventing unbounded RAM consumption.

---

## 7. What I Would Build Next
1. **Dynamic Blob Storage Separation (WiscKey Architecture)**: Storing large values in append-only `.vlog` files while keeping small keys in the LSM tree to reduce compaction write amplification to $<1.1$.
2. **Distributed Raft Consensus Integration**: Replicating WAL logs across a 3-node cluster for zero-RPO failover.
3. **Hardware Accelerators (io_uring & SPDK)**: Utilizing asynchronous `io_uring` kernel submission queues for ultra-high IOPs NVMe storage.
