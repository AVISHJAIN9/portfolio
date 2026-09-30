# AIML Model Cards: Double-Entry Financial Ledger

## Tier A: Transaction Anomaly Scoring model (Isolation Forest vs deterministic rule engine)
- **Baseline**: Non-AI Static Baseline
- **Target**: Statistically significant win on stated evaluation metric.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
- **Graceful Fallback**: Hardcoded static threshold rule engine.

## Tier B: Natural language to read-only ledger audit queries and reconciliation-break explainer (LLM never posts entries)
- **LLM Prompt Guard**: Strict schema validation. The LLM is NEVER on correctness or transactional paths.
