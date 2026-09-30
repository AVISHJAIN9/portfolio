# AIML Model Cards: LLM Serving Inference Scheduler

## Tier A: Output Length Predictor for continuous batching (vs FIFO scheduling baseline)
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: Cluster capacity planning assistant and latency SLA optimizer
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
