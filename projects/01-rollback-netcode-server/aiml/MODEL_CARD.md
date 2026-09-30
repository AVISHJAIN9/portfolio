# Model Card: Rollback Anti-Cheat Anomaly Detector (Tier A)

## Model Overview
- **Architecture**: Unsupervised Multi-Tree Isolation Forest (`sklearn.ensemble.IsolationForest`)
- **Version**: 1.2.0-reconciled
- **Input Dimensions**: 5 physical features (mouse dx variance, dy variance, angular velocity deg/s, motion entropy, reaction time ms)
- **Role**: Non-authoritative advisory triage queue for suspicious player sessions.
- **Safety Policy**: Does NOT disconnect players or alter game state. Flags replay sessions for human referee inspection.

## Performance & Baseline Comparison (Holdout Test Set N=2000, 10% Anomaly Rate)

| Metric | Non-ML Z-Score Baseline | Isolation Forest (Tier A) | Delta / Assessment |
|---|---|---|---|
| **ROC-AUC** | 0.9526 | **0.9420** | Non-trivial separation (< 1.0) |
| **PR-AUC** | 0.8182 | **0.7753** | Precision-Recall curve under class imbalance |
| **Precision** | 0.7857 | **0.8356** | **+4.99% higher precision** (fewer false bans) |
| **Recall** | 0.7150 | **0.6100** | High-confidence anomaly filtering |
| **F1-Score** | 0.7487 | **0.7052** | Tuned for minimal false accusations |
| **False Positive Rate (FPR)** | 2.17% | **1.33%** | **38.7% lower false positive rate** |
| **Inference Latency** | 0.0005 ms/sample | **0.0038 ms/sample** | 260,000 samples/sec per CPU core |

## Training & Validation Methodology
- **Dataset Generation**: Synthetic mouse kinematics with realistic overlapping Gaussian noise across human and automated aimbot/triggerbot profiles (`seed=42`).
- **Data Splitting**: Stratified 60% Train (N=6000), 20% Validation (N=2000), 20% Test (N=2000).
- **Threshold Selection**: Anomaly score threshold chosen strictly on the validation set to optimize F1 while constraining FPR < 1.5%.
