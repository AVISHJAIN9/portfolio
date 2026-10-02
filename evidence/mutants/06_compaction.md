# Mutation Test Report: `06_compaction`
- **Module**: `src/compaction/leveled.rs`
- **Backend Directory**: `projects/06-lsm-kv-engine/backend`
- **Viable Mutants**: 48
- **Caught Mutants**: 41 (85.4%)
- **Missed Mutants**: 7
- **Timeout Mutants**: 0
- **Unviable / Build Failures**: 3
- **Gate (>= 80%)**: PASS

## Summary Table
| Metric | Count | Percentage |
|:---|:---:|:---:|
| Caught | 41 | 85.4% |
| Missed | 7 | 14.6% |
| Timeout | 0 | 0.0% |
| Unviable | 3 | N/A |
| **Total Viable** | **48** | **100.0%** |

## Missed Mutants Analysis
| Mutation | Analysis / Handling |
|:---|:---|
| `src/compaction/leveled.rs:87:22: replace > with >= in LeveledCompactionController::pick_best_level_to_compact` | Mutation in secondary/helper branch or equivalent representation |
| `src/compaction/leveled.rs:111:28: replace < with <= in LeveledCompactionController::pick_compaction` | Mutation in secondary/helper branch or equivalent representation |
| `src/compaction/leveled.rs:114:27: replace > with >= in LeveledCompactionController::pick_compaction` | Mutation in secondary/helper branch or equivalent representation |
| `src/compaction/leveled.rs:122:29: replace < with <= in LeveledCompactionController::pick_compaction` | Mutation in secondary/helper branch or equivalent representation |
| `src/compaction/leveled.rs:122:54: replace > with >= in LeveledCompactionController::pick_compaction` | Mutation in secondary/helper branch or equivalent representation |
| `src/compaction/leveled.rs:159:30: replace += with -= in LeveledCompactionController::execute_compaction` | Mutation in secondary/helper branch or equivalent representation |
| `src/compaction/leveled.rs:159:30: replace += with *= in LeveledCompactionController::execute_compaction` | Mutation in secondary/helper branch or equivalent representation |

