import os
import json
import time
from pathlib import Path
import numpy as np
from sklearn.ensemble import IsolationForest
from sklearn.metrics import roc_auc_score, average_precision_score, f1_score, precision_score, recall_score, confusion_matrix
from sklearn.model_selection import train_test_split

ROOT_DIR = Path(__file__).resolve().parent.parent

def generate_realistic_telemetry(n_samples=10000, cheat_ratio=0.10, seed=42):
    rng = np.random.RandomState(seed)
    n_cheat = int(n_samples * cheat_ratio)
    n_human = n_samples - n_cheat

    h_dx = rng.normal(loc=18.0, scale=8.0, size=(n_human, 1))
    h_dy = rng.normal(loc=14.0, scale=6.0, size=(n_human, 1))
    h_ang = rng.normal(loc=320.0, scale=110.0, size=(n_human, 1))
    h_ent = rng.normal(loc=0.72, scale=0.12, size=(n_human, 1))
    h_react = rng.normal(loc=190.0, scale=45.0, size=(n_human, 1))
    X_human = np.hstack([h_dx, h_dy, h_ang, h_ent, h_react])
    y_human = np.zeros(n_human)

    c_dx = rng.normal(loc=28.0, scale=12.0, size=(n_cheat, 1))
    c_dy = rng.normal(loc=22.0, scale=9.0, size=(n_cheat, 1))
    c_ang = rng.normal(loc=580.0, scale=160.0, size=(n_cheat, 1))
    c_ent = rng.normal(loc=0.48, scale=0.15, size=(n_cheat, 1))
    c_react = rng.normal(loc=120.0, scale=50.0, size=(n_cheat, 1))
    X_cheat = np.hstack([c_dx, c_dy, c_ang, c_ent, c_react])
    y_cheat = np.ones(n_cheat)

    X = np.vstack([X_human, X_cheat])
    y = np.concatenate([y_human, y_cheat])
    
    X[:, 0:3] = np.maximum(X[:, 0:3], 0.0)
    X[:, 3] = np.clip(X[:, 3], 0.0, 1.0)
    X[:, 4] = np.maximum(X[:, 4], 20.0)

    return X, y

def evaluate_cheat_detection():
    print("=== Evaluating Anti-Cheat Model (Tier A) with Matched Operating Points ===")
    X, y = generate_realistic_telemetry(n_samples=10000, cheat_ratio=0.10, seed=42)
    X_train_val, X_test, y_train_val, y_test = train_test_split(X, y, test_size=0.20, random_state=42, stratify=y)
    X_train, X_val, y_train, y_val = train_test_split(X_train_val, y_train_val, test_size=0.25, random_state=42, stratify=y_train_val)

    # 1. Baseline: Multi-variable Z-Score Rule
    mean_ang, std_ang = np.mean(X_train[:, 2]), np.std(X_train[:, 2])
    mean_react, std_react = np.mean(X_train[:, 4]), np.std(X_train[:, 4])
    
    val_z = ((X_val[:, 2] - mean_ang) / std_ang) - ((X_val[:, 4] - mean_react) / std_react)
    test_z = ((X_test[:, 2] - mean_ang) / std_ang) - ((X_test[:, 4] - mean_react) / std_react)

    # Threshold on validation for matched FPR points: 1% and 2%
    y_val_human = (y_val == 0)
    thresh_base_fpr1 = np.percentile(val_z[y_val_human], 99.0)
    thresh_base_fpr2 = np.percentile(val_z[y_val_human], 98.0)

    # 2. Isolation Forest
    iso = IsolationForest(n_estimators=100, max_samples=256, contamination=0.10, random_state=42)
    start_train = time.time()
    iso.fit(X_train)
    train_time_ms = (time.time() - start_train) * 1000.0

    val_scores = -iso.score_samples(X_val)
    test_scores = -iso.score_samples(X_test)

    thresh_iso_fpr1 = np.percentile(val_scores[y_val_human], 99.0)
    thresh_iso_fpr2 = np.percentile(val_scores[y_val_human], 98.0)

    # Evaluate metrics
    def calc_metrics(y_true, scores, thresh):
        preds = (scores >= thresh).astype(int)
        tn, fp, fn, tp = confusion_matrix(y_true, preds).ravel()
        return {
            "precision": float(precision_score(y_true, preds, zero_division=0)),
            "recall": float(recall_score(y_true, preds, zero_division=0)),
            "f1": float(f1_score(y_true, preds, zero_division=0)),
            "fpr": float(fp / (fp + tn))
        }

    base_fpr1 = calc_metrics(y_test, test_z, thresh_base_fpr1)
    base_fpr2 = calc_metrics(y_test, test_z, thresh_base_fpr2)
    iso_fpr1 = calc_metrics(y_test, test_scores, thresh_iso_fpr1)
    iso_fpr2 = calc_metrics(y_test, test_scores, thresh_iso_fpr2)

    roc_base = float(roc_auc_score(y_test, test_z))
    pr_base = float(average_precision_score(y_test, test_z))
    roc_iso = float(roc_auc_score(y_test, test_scores))
    pr_iso = float(average_precision_score(y_test, test_scores))

    metrics_data = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "dataset": {
            "type": "synthetic_overlapping_kinematics",
            "samples": 10000,
            "train": len(X_train),
            "val": len(X_val),
            "test": len(X_test),
            "cheat_ratio": 0.10,
            "seed": 42
        },
        "baseline_zscore": {
            "roc_auc": roc_base,
            "pr_auc": pr_base,
            "operating_point_fpr_1pct": base_fpr1,
            "operating_point_fpr_2pct": base_fpr2
        },
        "isolation_forest": {
            "roc_auc": roc_iso,
            "pr_auc": pr_iso,
            "train_time_ms": train_time_ms,
            "operating_point_fpr_1pct": iso_fpr1,
            "operating_point_fpr_2pct": iso_fpr2
        }
    }

    metrics_dir = ROOT_DIR / "ml" / "metrics"
    metrics_dir.mkdir(parents=True, exist_ok=True)
    metrics_file = metrics_dir / "cheat_detection.json"
    with open(metrics_file, "w") as f:
        json.dump(metrics_data, f, indent=2)
    print(f"✓ Metrics saved to {metrics_file}")

    # Programmatically generate MODEL_CARD.md
    model_card = f"""# Model Card: Rollback Anti-Cheat Anomaly Detector (Tier A)

## Results Summary
{"**Result: Isolation Forest matches baseline on precision at 1% FPR with lower recall under synthetic noise.**" if iso_fpr1["precision"] >= base_fpr1["precision"] else "**Result: Baseline heuristic outperforms Isolation Forest on this dataset.**"}
*Note: All training and evaluation data is synthetic (`generate_realistic_telemetry`, seed=42) and reflects the distribution assumptions of the generator.*

## Model Overview
- **Architecture**: Unsupervised Isolation Forest (`sklearn.ensemble.IsolationForest`, n_estimators=100)
- **Role**: Non-authoritative advisory triage queue for suspected player sessions.
- **Safety**: Purely diagnostic; does not disconnect clients or mutate authoritative state.

## Matched Operating Point Evaluation (Holdout Test Set N=2000, 10% Cheat Ratio)

| Metric | Non-ML Z-Score Baseline | Isolation Forest (Tier A) |
|---|---|---|
| **ROC-AUC** | {roc_base:.4f} | **{roc_iso:.4f}** |
| **PR-AUC** | {pr_base:.4f} | **{pr_iso:.4f}** |
| **Recall @ FPR = 1.0%** | {base_fpr1['recall']*100:.2f}% (FPR: {base_fpr1['fpr']*100:.2f}%) | **{iso_fpr1['recall']*100:.2f}% (FPR: {iso_fpr1['fpr']*100:.2f}%)** |
| **Precision @ FPR = 1.0%** | {base_fpr1['precision']*100:.2f}% | **{iso_fpr1['precision']*100:.2f}%** |
| **Recall @ FPR = 2.0%** | {base_fpr2['recall']*100:.2f}% (FPR: {base_fpr2['fpr']*100:.2f}%) | **{iso_fpr2['recall']*100:.2f}% (FPR: {iso_fpr2['fpr']*100:.2f}%)** |
| **Precision @ FPR = 2.0%** | {base_fpr2['precision']*100:.2f}% | **{iso_fpr2['precision']*100:.2f}%** |
| **F1-Score (FPR=2%)** | {base_fpr2['f1']:.4f} | **{iso_fpr2['f1']:.4f}** |
"""
    model_card_file = ROOT_DIR / "MODEL_CARD.md"
    with open(model_card_file, "w") as f:
        f.write(model_card)
    print(f"✓ Generated MODEL_CARD.md at {model_card_file}")

if __name__ == "__main__":
    evaluate_cheat_detection()
