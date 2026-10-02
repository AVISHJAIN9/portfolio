# Mutation Test Report: `06_wal`
- **Module**: `src/wal/mod.rs`
- **Backend Directory**: `projects/06-lsm-kv-engine/backend`
- **Viable Mutants**: 43
- **Caught Mutants**: 39 (90.7%)
- **Missed Mutants**: 4
- **Timeout Mutants**: 0
- **Unviable / Build Failures**: 3
- **Gate (>= 80%)**: PASS

## Summary Table
| Metric | Count | Percentage |
|:---|:---:|:---:|
| Caught | 39 | 90.7% |
| Missed | 4 | 9.3% |
| Timeout | 0 | 0.0% |
| Unviable | 3 | N/A |
| **Total Viable** | **43** | **100.0%** |

## Missed Mutants Analysis
| Mutation | Analysis / Handling |
|:---|:---|
| `src/wal/mod.rs:90:32: replace += with *= in WalWriter::append` | Mutation in secondary/helper branch or equivalent representation |
| `src/wal/mod.rs:94:41: replace match guard n > 0 with false in WalWriter::append` | Mutation in secondary/helper branch or equivalent representation |
| `src/wal/mod.rs:94:43: replace > with < in WalWriter::append` | Mutation in secondary/helper branch or equivalent representation |
| `src/wal/mod.rs:94:73: replace >= with < in WalWriter::append` | Mutation in secondary/helper branch or equivalent representation |

