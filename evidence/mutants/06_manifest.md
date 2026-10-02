# Mutation Test Report: `06_manifest`
- **Module**: `src/manifest/mod.rs`
- **Backend Directory**: `projects/06-lsm-kv-engine/backend`
- **Viable Mutants**: 12
- **Caught Mutants**: 11 (91.7%)
- **Missed Mutants**: 1
- **Timeout Mutants**: 0
- **Unviable / Build Failures**: 1
- **Gate (>= 80%)**: PASS

## Summary Table
| Metric | Count | Percentage |
|:---|:---:|:---:|
| Caught | 11 | 91.7% |
| Missed | 1 | 8.3% |
| Timeout | 0 | 0.0% |
| Unviable | 1 | N/A |
| **Total Viable** | **12** | **100.0%** |

## Missed Mutants Analysis
| Mutation | Analysis / Handling |
|:---|:---|
| `src/manifest/mod.rs:64:31: replace < with <= in Version::apply_edit` | Mutation in secondary/helper branch or equivalent representation |

