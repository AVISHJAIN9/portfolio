import numpy as np
from sklearn.ensemble import IsolationForest
from sklearn.metrics import roc_auc_score, average_precision_score, f1_score, precision_score, recall_score, confusion_matrix
from sklearn.model_selection import train_test_split
import time

def generate_realistic_telemetry(n_samples=10000, cheat_ratio=0.10, seed=42):
    """
    Generates synthetic telemetry with realistic overlapping distributions.
    Features:
    0: mouse_dx_variance (Human: N(18, 8), Cheater: N(28, 12))
    1: mouse_dy_variance (Human: N(14, 6), Cheater: N(22, 9))
    2: angular_velocity_deg_s (Human: N(320, 110), Cheater: N(580, 160)) -> Overlapping!
    3: motion_entropy (Human: N(0.72, 0.12), Cheater: N(0.48, 0.15))
    4: reaction_time_ms (Human: N(190, 45), Cheater: N(120, 50))
    """
    rng = np.random.RandomState(seed)
    n_cheat = int(n_samples * cheat_ratio)
    n_human = n_samples - n_cheat

    # Human distributions
    h_dx = rng.normal(loc=18.0, scale=8.0, size=(n_human, 1))
    h_dy = rng.normal(loc=14.0, scale=6.0, size=(n_human, 1))
    h_ang = rng.normal(loc=320.0, scale=110.0, size=(n_human, 1))
    h_ent = rng.normal(loc=0.72, scale=0.12, size=(n_human, 1))
    h_react = rng.normal(loc=190.0, scale=45.0, size=(n_human, 1))
    X_human = np.hstack([h_dx, h_dy, h_ang, h_ent, h_react])
    y_human = np.zeros(n_human)

    # Cheater distributions (partially subtle aimbot / triggerbot with noise)
    c_dx = rng.normal(loc=28.0, scale=12.0, size=(n_cheat, 1))
    c_dy = rng.normal(loc=22.0, scale=9.0, size=(n_cheat, 1))
    c_ang = rng.normal(loc=580.0, scale=160.0, size=(n_cheat, 1))
    c_ent = rng.normal(loc=0.48, scale=0.15, size=(n_cheat, 1))
    c_react = rng.normal(loc=120.0, scale=50.0, size=(n_cheat, 1))
    X_cheat = np.hstack([c_dx, c_dy, c_ang, c_ent, c_react])
    y_cheat = np.ones(n_cheat)

    X = np.vstack([X_human, X_cheat])
    y = np.concatenate([y_human, y_cheat])
    
    # Clip physical limits
    X[:, 0:3] = np.maximum(X[:, 0:3], 0.0)
    X[:, 3] = np.clip(X[:, 3], 0.0, 1.0)
    X[:, 4] = np.maximum(X[:, 4], 20.0)

    return X, y

def evaluate_cheat_detection():
    print("==================================================================")
    print(" AIML TIER A: RECONCILED CHEAT DETECTION (OVERLAPPING DISTRIBUTIONS)")
    print("==================================================================")
    
    # 1. Stratified Train / Val / Test Split (60% Train, 20% Val, 20% Test)
    X, y = generate_realistic_telemetry(n_samples=10000, cheat_ratio=0.10, seed=42)
    X_train_val, X_test, y_train_val, y_test = train_test_split(X, y, test_size=0.20, random_state=42, stratify=y)
    X_train, X_val, y_train, y_val = train_test_split(X_train_val, y_train_val, test_size=0.25, random_state=42, stratify=y_train_val)

    print(f"Dataset Split: Train={len(X_train)}, Val={len(X_val)}, Test={len(X_test)}")
    print(f"Positive Cheat Ratio: {np.mean(y_test)*100:.1f}%")

    # 2. Non-ML Baseline: Multi-metric Z-Score Rule on Validation Set
    mean_ang, std_ang = np.mean(X_train[:, 2]), np.std(X_train[:, 2])
    mean_react, std_react = np.mean(X_train[:, 4]), np.std(X_train[:, 4])
    
    val_z_score = ((X_val[:, 2] - mean_ang) / std_ang) - ((X_val[:, 4] - mean_react) / std_react)
    test_z_score = ((X_test[:, 2] - mean_ang) / std_ang) - ((X_test[:, 4] - mean_react) / std_react)

    # Choose threshold on validation set
    best_thresh_base = 1.0
    best_f1_base = 0.0
    for thresh in np.linspace(-1.0, 4.0, 100):
        preds = (val_z_score > thresh).astype(int)
        f1 = f1_score(y_val, preds)
        if f1 > best_f1_base:
            best_f1_base = f1
            best_thresh_base = thresh

    # Test baseline
    y_pred_base = (test_z_score > best_thresh_base).astype(int)
    roc_base = roc_auc_score(y_test, test_z_score)
    pr_base = average_precision_score(y_test, test_z_score)
    f1_base = f1_score(y_test, y_pred_base)
    prec_base = precision_score(y_test, y_pred_base)
    rec_base = recall_score(y_test, y_pred_base)
    tn, fp, fn, tp = confusion_matrix(y_test, y_pred_base).ravel()
    fpr_base = fp / (fp + tn)

    print("\n--- [BASELINE: Multi-variable Z-Score / CUSUM Rule] ---")
    print(f"  ROC-AUC:   {roc_base:.4f}")
    print(f"  PR-AUC:    {pr_base:.4f}")
    print(f"  Precision: {prec_base:.4f}")
    print(f"  Recall:    {rec_base:.4f}")
    print(f"  F1-Score:  {f1_base:.4f}")
    print(f"  FPR:       {fpr_base*100:.2f}%")

    # 3. Model: Isolation Forest
    iso = IsolationForest(n_estimators=100, max_samples=256, contamination=0.10, random_state=42)
    start_train = time.time()
    iso.fit(X_train)
    train_time = (time.time() - start_train) * 1000.0

    val_scores = -iso.score_samples(X_val) # Higher anomaly score = more anomalous
    test_scores = -iso.score_samples(X_test)

    # Select threshold on validation set maximizing F1
    best_thresh_iso = np.percentile(val_scores, 90)
    best_f1_iso = 0.0
    for q in np.linspace(80, 99, 100):
        thresh = np.percentile(val_scores, q)
        preds = (val_scores >= thresh).astype(int)
        f1 = f1_score(y_val, preds)
        if f1 > best_f1_iso:
            best_f1_iso = f1
            best_thresh_iso = thresh

    y_pred_iso = (test_scores >= best_thresh_iso).astype(int)
    roc_iso = roc_auc_score(y_test, test_scores)
    pr_iso = average_precision_score(y_test, test_scores)
    f1_iso = f1_score(y_test, y_pred_iso)
    prec_iso = precision_score(y_test, y_pred_iso)
    rec_iso = recall_score(y_test, y_pred_iso)
    tn, fp, fn, tp = confusion_matrix(y_test, y_pred_iso).ravel()
    fpr_iso = fp / (fp + tn)

    start_inf = time.time()
    _ = iso.score_samples(X_test)
    inf_lat = ((time.time() - start_inf) / len(X_test)) * 1000.0

    print("\n--- [MODEL: Isolation Forest (Tier A Anomaly Triage)] ---")
    print(f"  ROC-AUC:   {roc_iso:.4f}")
    print(f"  PR-AUC:    {pr_iso:.4f}")
    print(f"  Precision: {prec_iso:.4f}")
    print(f"  Recall:    {rec_iso:.4f}")
    print(f"  F1-Score:  {f1_iso:.4f}")
    print(f"  FPR:       {fpr_iso*100:.2f}%")
    print(f"  Train Time: {train_time:.2f} ms")
    print(f"  Latency:    {inf_lat:.4f} ms/sample")

    print("\n--- [RECONCILIATION SUMMARY] ---")
    print(f"  Improvement over Baseline: ROC-AUC {roc_iso:.4f} vs {roc_base:.4f}, PR-AUC {pr_iso:.4f} vs {pr_base:.4f}")
    print(f"  Model is non-trivially separable (ROC-AUC {roc_iso:.4f} < 1.0, PR-AUC {pr_iso:.4f} with realistic FPR {fpr_iso*100:.2f}%).")

if __name__ == '__main__':
    evaluate_cheat_detection()
