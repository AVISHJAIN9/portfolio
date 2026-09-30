# `pf-errors`

## Purpose
Provides unified typed error definitions (`PlatformError`), result aliases (`PlatformResult<T>`), and RFC 7807 Problem Details serialization across all Portfolio Forge engine crates.

## Invariants & Guarantees
- All variants are serializable via `serde` for RPC / distributed wire transfer.
- Conversions to `ProblemDetails` map cleanly to standard HTTP status codes (404, 401, 422, 429, 409, 500).
- Zero panics in error formatting.
