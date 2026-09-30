# AIML Model Cards: Low-Latency HLS Origin Server

## Tier A: Player Rebuffering Predictor (LightGBM vs fixed buffer threshold)
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: Stream incident root-cause summarizer and ABR ladder tuning advisor
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
