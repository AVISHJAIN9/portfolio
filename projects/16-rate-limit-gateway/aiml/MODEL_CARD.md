# AIML Model Cards: Distributed Rate-Limiting API Gateway

## Tier A: Abuse & DDoS Traffic Anomaly Detector (Isolation Forest vs static threshold)
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: 'Why was this client throttled?' forensic explainer assistant
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
