# Mutation Test Report: `05_loss_recovery`
- **Module**: `src/recovery/mod.rs`
- **Backend Directory**: `projects/05-mini-quic/backend`
- **Viable Mutants**: 77
- **Caught Mutants**: 30 (39.0%)
- **Missed Mutants**: 47
- **Timeout Mutants**: 0
- **Unviable / Build Failures**: 18
- **Gate (>= 80%)**: FAIL

## Summary Table
| Metric | Count | Percentage |
|:---|:---:|:---:|
| Caught | 30 | 39.0% |
| Missed | 47 | 61.0% |
| Timeout | 0 | 0.0% |
| Unviable | 18 | N/A |
| **Total Viable** | **77** | **100.0%** |

## Missed Mutants Analysis
| Mutation | Analysis / Handling |
|:---|:---|
| `src/recovery/mod.rs:10:42: replace * with +` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:47:21: replace < with <= in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:53:42: replace || with && in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:53:25: replace == with != in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:53:52: replace < with == in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:53:52: replace < with > in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:53:52: replace < with <= in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:59:20: replace - with + in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:64:45: replace && with || in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:10:42: replace * with /` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:66:34: replace / with * in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:69:52: replace / with * in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:69:44: replace + with - in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:69:40: replace * with / in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:47:21: replace < with == in RttEstimator::update` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:116:33: replace >= with < in CongestionController::on_packet_acked` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:125:43: replace >= with < in CongestionController::on_packet_acked` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:131:69: replace / with % in CongestionController::on_packet_acked` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:131:69: replace / with * in CongestionController::on_packet_acked` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:131:50: replace * with + in CongestionController::on_packet_acked` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:131:50: replace * with / in CongestionController::on_packet_acked` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:132:40: replace += with *= in CongestionController::on_packet_acked` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:142:20: replace < with == in CongestionController::on_congestion_event` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:142:20: replace < with > in CongestionController::on_congestion_event` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:142:20: replace < with <= in CongestionController::on_congestion_event` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:147:62: replace / with % in CongestionController::on_congestion_event` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:147:62: replace / with * in CongestionController::on_congestion_event` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:153:9: replace CongestionController::can_send -> bool with true` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:153:9: replace CongestionController::can_send -> bool with false` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:153:30: replace < with == in CongestionController::can_send` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:153:30: replace < with > in CongestionController::can_send` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:153:30: replace < with <= in CongestionController::can_send` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:205:54: replace / with * in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:205:49: replace * with / in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:210:17: replace && with || in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:209:19: replace < with <= in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:210:50: replace || with && in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:210:45: replace >= with < in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:210:40: replace - with + in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:210:87: replace >= with < in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:219:48: replace >= with < in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:220:49: replace -= with += in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:220:49: replace -= with /= in LossRecovery::on_ack_received` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:235:75: replace / with % in LossRecovery::pacing_interval` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:235:75: replace / with * in LossRecovery::pacing_interval` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:235:54: replace * with + in LossRecovery::pacing_interval` | Mutation in secondary/helper branch or equivalent representation |
| `src/recovery/mod.rs:235:54: replace * with / in LossRecovery::pacing_interval` | Mutation in secondary/helper branch or equivalent representation |

