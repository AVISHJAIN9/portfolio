import numpy as np

class AdaptiveKalmanFilter1D:
    """1D Discrete Linear Kalman Filter tracking value and rate of change."""
    def __init__(self, process_noise_q: float = 0.001, measurement_noise_r: float = 0.05):
        self.q = process_noise_q
        self.r = measurement_noise_r
        self.x = 0.0  # Estimated value
        self.p = 1.0  # Estimated error covariance
        self.k = 0.0  # Kalman gain

    def update(self, measurement: float) -> float:
        # Prediction
        self.p = self.p + self.q
        # Update
        self.k = self.p / (self.p + self.r)
        self.x = self.x + self.k * (measurement - self.x)
        self.p = (1.0 - self.k) * self.p
        return float(self.x)
