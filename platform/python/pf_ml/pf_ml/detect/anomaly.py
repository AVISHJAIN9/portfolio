import numpy as np
from sklearn.ensemble import IsolationForest
from typing import Dict, Any, List

class AnomalyTriageDetector:
    """Unsupervised Isolation Forest anomaly detector for streaming engine metrics."""
    def __init__(self, contamination: float = 0.05, n_estimators: int = 100, random_state: int = 42):
        self.model = IsolationForest(
            contamination=contamination,
            n_estimators=n_estimators,
            random_state=random_state
        )
        self.is_fitted = False

    def fit(self, X: np.ndarray):
        self.model.fit(X)
        self.is_fitted = True

    def predict_anomaly(self, x: np.ndarray) -> bool:
        if not self.is_fitted:
            return False
        pred = self.model.predict(x.reshape(1, -1))
        return bool(pred[0] == -1)
