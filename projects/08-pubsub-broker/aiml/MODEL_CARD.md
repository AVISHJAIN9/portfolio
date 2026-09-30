# AIML Model Cards: Distributed Partitioned PubSub Broker

## Tier A: Consumer Lag Forecasting and Auto-Scaling Advisor (vs static lag threshold)
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: Natural language to topic schema & partition topology configuration advisor
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
