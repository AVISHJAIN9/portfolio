# AIML Model Cards: Columnar Query Engine (DuckDB Style)

## Tier A: Learned Cardinality Estimator (vs histogram baseline; q-error evaluation)
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: Natural language to SQL parser with physical execution plan cost preview
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
