# Mutation Test Report: `05_frame_codec`
- **Module**: `src/wire/frame.rs`
- **Backend Directory**: `projects/05-mini-quic/backend`
- **Viable Mutants**: 70
- **Caught Mutants**: 45 (64.3%)
- **Missed Mutants**: 25
- **Timeout Mutants**: 0
- **Unviable / Build Failures**: 1
- **Gate (>= 80%)**: FAIL

## Summary Table
| Metric | Count | Percentage |
|:---|:---:|:---:|
| Caught | 45 | 64.3% |
| Missed | 25 | 35.7% |
| Timeout | 0 | 0.0% |
| Unviable | 1 | N/A |
| **Total Viable** | **70** | **100.0%** |

## Missed Mutants Analysis
| Mutation | Analysis / Handling |
|:---|:---|
| `src/wire/frame.rs:149:13: delete match arm Frame::ResetStream{stream_id, error_code, final_size} in Frame::encode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:155:13: delete match arm Frame::StopSending{stream_id, error_code} in Frame::encode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:187:13: delete match arm Frame::HandshakeDone in Frame::encode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:134:34: replace > with >= in Frame::encode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:143:34: replace > with >= in Frame::encode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:245:13: delete match arm 0x04 in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:251:13: delete match arm 0x05 in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:323:13: delete match arm 0x1e in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:106:13: delete match arm Frame::Ping in Frame::encode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:216:38: replace || with && in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:216:32: replace > with == in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:216:32: replace > with >= in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:216:57: replace < with == in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:216:57: replace < with <= in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:216:80: replace * with + in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:216:80: replace * with / in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:259:36: replace < with > in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:267:45: replace & with | in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:267:45: replace & with ^ in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:272:42: replace & with | in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:272:42: replace & with ^ in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:277:36: replace < with > in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:280:39: replace & with | in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:280:39: replace & with ^ in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |
| `src/wire/frame.rs:313:36: replace < with > in Frame::decode` | Mutation in secondary/helper branch or equivalent representation |

