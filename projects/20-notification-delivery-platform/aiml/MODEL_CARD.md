# AIML Model Cards: Multi-Provider Notification Platform

## Tier A: Send-time & optimal delivery channel predictor; provider failure predictor
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: Notification template drafter and provider failover incident explainer
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
