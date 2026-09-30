# AIML Model Card: Learned Vector Query Planner
- **Intended Use**: Predict whether a filtered vector query should execute via In-Graph Beam Search, Post-Filtering, or Brute-Force scanning.
- **Baseline**: Static selectivity thresholds (< 0.1% Brute Force, 0.1% - 15% In-Graph, > 15% Post-Filter).
- **Target Metric**: QPS improvement >= 25% over baseline.
- **Status**: Specified & Scaffolded (**TO BE MEASURED**).
