# Project 05: Mini-QUIC — Interview Notes & System Design

## 1. 60-Second Pitch
**Mini-QUIC** is an RFC 9000 & RFC 9001 compliant userspace QUIC transport protocol implementation written from scratch in pure Rust. It features an integrated in-tree TLS 1.3 cryptographic handshake (RFC 8446), ChaCha20-Poly1305 / AES-128-GCM authenticated encryption, RFC 9002 loss recovery with adaptive RTT tracking, and CUBIC congestion control with multi-event ECN suppression. Verified through live UDP socket interoperability against the reference Python implementation `aioquic` v1.3.0 over 1 MiB pseudo-random payloads, it achieves 100% SHA-256 payload integrity across lossy network conditions (0% to 5% packet loss) and passes a rigorous suite of 4 cryptographic and protocol negative controls.

---

## 2. Architecture Walk-Through
The transport engine operates as a stateful, zero-heap-allocation packet processing pipeline:

```
[UDP Datagram Socket / Network Simulator]
   │
   ▼ [projects/05-mini-quic/backend/src/codec/packet.rs]
[Zero-Alloc Header & Packet Parser (QuicVarInt, Packet Number Decoding)]
   │
   ▼ [projects/05-mini-quic/backend/src/crypto/handshake.rs & pf-crypto]
[TLS 1.3 Handshake State Machine & 1-RTT AEAD Decryption]
   │
   ▼ [projects/05-mini-quic/backend/src/recovery/loss.rs & ack.rs]
[ACK Processor, Adaptive RTT Smoothing, & Loss Detection (PTO / 3-Packet Threshold)]
   │
   ▼ [projects/05-mini-quic/backend/src/congestion/cubic.rs & ecn.rs]
[CUBIC Congestion Control Engine (Multi-Event ECN Suppression per RTT)]
   │
   ▼ [projects/05-mini-quic/backend/src/stream/mod.rs & flow_control.rs]
[Stream Manager & Multi-Stream Flow Control Windows]
   │
   ▼ [projects/05-mini-quic/backend/src/path/mod.rs]
[Multi-Path State Management & Path Challenge / Response Validation]
```

### Key Source Components:
- **Packet Codec & Wire Formats**: [`src/codec/packet.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/codec/packet.rs) implements RFC 9000 Initial, Handshake, and 1-RTT packet parsing with zero allocations on the datagram path.
- **TLS 1.3 Cryptographic Handshake**: [`src/crypto/handshake.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/crypto/handshake.rs) implements client/server key exchange using X25519, HKDF-SHA256 secret derivation, and Finished verification.
- **Loss Recovery & Congestion**: [`src/recovery/loss.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/recovery/loss.rs) and [`src/congestion/cubic.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/congestion/cubic.rs) implement RFC 9002 RTT smoothing, PTO backoff, and CUBIC window growth.
- **Deterministic Network Simulator**: [`src/simulator/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/simulator/mod.rs) provides deterministic virtual-clock simulation of latency, loss, duplicate, and reorder events.
- **Path Management & Migration**: [`src/path/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/path/mod.rs) tracks path validation via `PATH_CHALLENGE`/`PATH_RESPONSE` per RFC 9000 §8.2.

---

## 3. The 5 Hardest Bugs Actually Found & Fixed
1. **AEAD Packet Number Decryption Mask Underflow**
   - *Symptom*: Interop with `aioquic` failed on the first 1-RTT packet because header protection sample offset was calculated from the payload start rather than 4 bytes into the ciphertext.
   - *Fix*: Aligned sample extraction with RFC 9001 §5.4.2 in `153705e`, verified against authentic aioquic ciphertext.
2. **ECN Multi-Event Window Collapse**
   - *Symptom*: Receiving multiple CE (Congestion Experienced) packets within a single RTT caused repeated multiplicative window decreases, collapsing throughput to 1 MSS.
   - *Fix*: Added congestion epoch tracking in [`src/congestion/ecn.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/congestion/ecn.rs) in `14aeca4`, limiting window reduction to at most once per RTT.
3. **Loss Recovery False Loss Detection Under High Jitter**
   - *Symptom*: 3-packet threshold triggered spurious retransmissions when UDP packets arrived out-of-order by 4 positions under 20ms jitter.
   - *Fix*: Integrated time-threshold loss detection ($9/8 \times \max(SRTT, latest\_RTT)$) per RFC 9002 §6.1.2.
4. **VarInt Proptest Truncation Panic on 8-Byte Boundary**
   - *Symptom*: Property-based testing revealed integer overflow when decoding a 62-bit VarInt whose two MSBs were `0b11` and lower bytes contained `0xFFFFFFFFFFFFFFFF`.
   - *Fix*: Masked out the 2 MSB tag bits before bitshifting in `QuicVarInt::decode`.
5. **Lossy Proxy Source Port Spoofing Migration Flaw**
   - *Symptom*: A single spoofed UDP packet from an alternate IP triggered an immediate active path switch before path validation completed.
   - *Fix*: Implemented RFC 9000 §8.2 path state machine in [`src/path/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/path/mod.rs), requiring matching 8-byte `PATH_RESPONSE` before activating migrated paths.

---

## 4. 8 Key Architectural Decisions & Trade-Offs (from `docs/DECISIONS.md`)
1. **Rust for Zero-Cost Transport Performance (ADR-002)**: Guarantees memory safety without garbage collector pauses affecting packet pacing.
2. **Deterministic Virtual Clock & Network Simulator (ADR-005, 05-064)**: Enables 100% reproducible test traces with fixed RNG seeds, eliminating flaky timing tests.
3. **Zero-Heap-Allocation Packet Parsing (ADR-011)**: Operates directly on byte slices for packet headers and frame decoding, preventing heap fragmentation under 100k pps.
4. **RFC 9002 Loss Recovery & CUBIC CC (ADR-012)**: Implements standard congestion control algorithms matching Linux kernel CUBIC dynamics.
5. **In-Tree TLS 1.3 Handshake Implementation (ADR-027)**: Direct protocol integration using `pf-crypto` primitives, avoiding heavyweight external OpenSSL/BoringSSL C-bindings.
6. **Multi-Path Validation State Machine (05-058 to 05-063)**: Explicit 4-state lifecycle (`New`, `Validating`, `Validated`, `Failed`) ensuring anti-amplification limits on unvalidated paths.
7. **Buffer Pooling with Zero Steady-State Allocations (05-071)**: Reusable packet buffer rings achieving $>99\%$ reuse ratio under continuous transfer.
8. **Reproducible Builds & Cargo-Deny Auditing (05-144, 05-145)**: Pinned dependency licenses (MIT/Apache-2.0) and deterministic bit-identical binary generation.

---

## 5. System Limitations & Scope Boundaries
- **Supported Scope**: RFC 9000 transport framing, TLS 1.3 handshake, 1-RTT AEAD encryption, loss recovery, CUBIC CC, ECN processing, multi-path validation, deterministic network simulator, buffer pooling, reproducible builds.
- **Descoped / Excluded**: HTTP/3 QPACK header compression, web browser WebTransport bindings, BPF kernel-bypass XDP drivers, TLS session resumption tickets (0-RTT early data).
- **Manual Verification Dependency**: `05-142` (Automated GitHub Actions benchmark runner) requires live cloud runner integration.

---

## 6. 15 Likely Interview Questions & Code-Grounded Answers

1. **How does QUIC solve TCP's head-of-line blocking problem?**
   - *Answer*: In QUIC, independent streams multiplex over a single UDP connection. As shown in [`src/stream/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/stream/mod.rs), dropping a packet for Stream A only stalls Stream A; Stream B continues delivering data to the application immediately.

2. **How does the TLS 1.3 handshake integrate with QUIC packet protection?**
   - *Answer*: Unlike TLS over TCP which runs as a stream payload, QUIC embeds TLS handshake messages in `CRYPTO` frames across separate packet encryption levels (Initial, Handshake, 1-RTT) as implemented in [`src/crypto/handshake.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/crypto/handshake.rs).

3. **How do you calculate RTT and detect packet loss without duplicate ACKs?**
   - *Answer*: Every QUIC packet number is monotonically increasing. In [`src/recovery/loss.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/recovery/loss.rs), we compute $SRTT = \frac{7}{8}SRTT + \frac{1}{8}latest\_rtt$ and declare a packet lost if an ACK arrives for a packet sent $\ge 3$ packet numbers later or after $9/8 \times \max(SRTT, latest\_rtt)$.

4. **How does CUBIC congestion control determine the congestion window?**
   - *Answer*: In [`src/congestion/cubic.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/congestion/cubic.rs), window growth follows a cubic function of elapsed time $t$ since the last loss: $W(t) = C(t - K)^3 + W_{max}$, providing concave growth near $W_{max}$ and convex probing beyond.

5. **How does the engine handle Explicit Congestion Notification (ECN)?**
   - *Answer*: In [`src/congestion/ecn.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/congestion/ecn.rs), when an incoming ACK reports an increase in the ECN-CE counter, we trigger congestion response at most once per RTT epoch to prevent window collapse.

6. **How does path validation work during connection migration?**
   - *Answer*: In [`src/path/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/path/mod.rs), the endpoint generates an 8-byte cryptographic challenge in a `PATH_CHALLENGE` frame. The peer must return the exact 8 bytes in a `PATH_RESPONSE`. Migration is committed only after validation succeeds.

7. **How do you defend against amplification attacks on unvalidated paths?**
   - *Answer*: Per RFC 9000 §8.1, an endpoint must not send more than $3\times$ the received bytes to an unvalidated peer address. We enforce this in [`src/path/mod.rs:85`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/path/mod.rs#L85) via `anti_amplification_budget`.

8. **How is zero-allocation packet parsing achieved?**
   - *Answer*: `PacketParser` in [`src/codec/packet.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/codec/packet.rs) borrows byte slices directly from the incoming UDP datagram buffer without allocating `Vec` or string instances.

9. **How do you test the transport deterministically?**
   - *Answer*: [`src/simulator/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/simulator/mod.rs) uses an injected `VirtualClock` and seeded PRNG. Given the same seed, every simulated packet event and trace SHA-256 is 100% bit-identical.

10. **How does the Buffer Pool manage memory under sustained load?**
    - *Answer*: [`src/runtime/mod.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/runtime/mod.rs) maintains a pool of pre-allocated 64 KiB buffers with RAII release guards, maintaining zero steady-state heap allocations during data streaming.

11. **How does the engine protect against spoofed packet numbers?**
    - *Answer*: In [`src/codec/packet.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/codec/packet.rs), packet number lengths are decoded using header protection masks and verified against the largest acknowledged packet window.

12. **How are stream flow control limits negotiated and enforced?**
    - *Answer*: In [`src/stream/flow_control.rs`](file:///Users/avishjain/avishjain9-repos/portfolio/projects/05-mini-quic/backend/src/stream/flow_control.rs), connections track maximum data limits advertised via `MAX_DATA` and `MAX_STREAM_DATA` frames, applying immediate backpressure if limits are reached.

13. **How do you test interoperability against real external implementations?**
    - *Answer*: `tests/interop/run_interop.py` spawns Python's `aioquic` v1.3.0 in both client and server roles, transferring a 1 MiB pseudo-random payload through a lossy UDP proxy, validating SHA-256 digests.

14. **What negative controls are tested in the interop suite?**
    - *Answer*: We verify: (1) ALPN mismatch terminates handshake, (2) 1-RTT bit-flip corrupts AEAD tag and triggers retransmission, (3) 100% loss causes PTO expiration, and (4) payload tampering triggers integrity error.

15. **How are reproducible builds guaranteed?**
    - *Answer*: The `make repro` target compiles release binaries with `--remap-path-prefix` and static environment clamping, asserting bit-for-bit identical SHA-256 hashes across isolated build directories.

---

## 7. What I Would Build Next
1. **0-RTT Early Data Support (`05-020`)**: TLS 1.3 session ticket caching with anti-replay protection.
2. **HTTP/3 & QPACK Layer (`05-081`)**: Dynamic table encoder and decoder for HTTP/3 request/response multiplexing.
3. **Kernel-Bypass XDP Socket Backend**: AF_XDP socket integration for sub-microsecond packet ingestion at 10 Gbps line rate.
