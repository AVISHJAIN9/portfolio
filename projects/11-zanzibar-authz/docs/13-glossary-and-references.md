# 13 Glossary And References: Zanzibar Authorization Engine

## 1. Specification Overview [Specified]
Zanzibar-style global relationship-based access control with tuple rewrites, zookies, and snapshot-aware cache.

## 2. Core Mechanisms [Specified]
Userset subjects, namespace rewrites (union, intersection, exclusion, tuple-to-userset), parallel Check with cycle/depth limits, zookies against new-enemy problem, MVCC storage, changelog for Watch, snapshot cache with single-flight coalescing.

## 3. Unknowns & Benchmarks
All performance numbers, throughput rates, and latency figures are marked **TO BE MEASURED** until run on standard hardware.
