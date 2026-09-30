# AIML Model Cards: Distributed Cache with Pluggable Eviction

## Tier A: Learned Eviction Policy model evaluated on public trace datasets vs ARC/TinyLFU; hot-key predictor
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: Natural language cache diagnostic and cache-miss analysis assistant
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
