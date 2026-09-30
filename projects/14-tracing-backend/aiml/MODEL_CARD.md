# AIML Model Cards: Distributed Tracing with Tail Sampling

## Tier A: Learned Tail-Sampling Policy vs rule-based sampling; latency anomaly detector
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: Distributed trace root-cause summarizer and bottleneck explainer
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
