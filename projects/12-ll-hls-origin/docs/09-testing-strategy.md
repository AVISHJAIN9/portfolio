# 09 Testing Strategy: Low-Latency HLS Origin Server

## 1. Specification Overview [Specified]
Sub-3-second glass-to-glass Low-Latency HLS origin server with CMAF chunking and delta playlists.

## 2. Core Mechanisms [Specified]
RTMP/SRT ingest, CMAF fragments, ~200 ms partial segments, blocking playlist reload, delta playlists, coalescing 10k waiting viewers into one wait/notify, FFmpeg 3-rendition ABR ladder, DVR retention and eviction, CDN cache-header design.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
