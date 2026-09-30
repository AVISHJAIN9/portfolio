# Model Card: Rollback Anti-Cheat Anomaly Detector (Tier A)

## Results Summary
**Result: Isolation Forest matches baseline on precision at 1% FPR with lower recall under synthetic noise.**
*Note: All training and evaluation data is synthetic (`generate_realistic_telemetry`, seed=42) and reflects the distribution assumptions of the generator.*

## Model Overview
- **Architecture**: Unsupervised Isolation Forest (`sklearn.ensemble.IsolationForest`, n_estimators=100)
- **Role**: Non-authoritative advisory triage queue for suspected player sessions.
- **Safety**: Purely diagnostic; does not disconnect clients or mutate authoritative state.

## Matched Operating Point Evaluation (Holdout Test Set N=2000, 10% Cheat Ratio)

| Metric | Non-ML Z-Score Baseline | Isolation Forest (Tier A) |
|---|---|---|
| **ROC-AUC** | 0.9526 | **0.9420** |
| **PR-AUC** | 0.8182 | **0.7753** |
| **Recall @ FPR = 1.0%** | 59.00% (FPR: 0.89%) | **50.00% (FPR: 0.61%)** |
| **Precision @ FPR = 1.0%** | 88.06% | **90.09%** |
| **Recall @ FPR = 2.0%** | 66.00% (FPR: 1.50%) | **61.00% (FPR: 1.72%)** |
| **Precision @ FPR = 2.0%** | 83.02% | **79.74%** |
| **F1-Score (FPR=2%)** | 0.7354 | **0.6912** |
