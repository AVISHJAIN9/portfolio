import numpy as np
from typing import Dict, List, Any

def generate_synthetic_telemetry(n_samples: int = 1000, seed: int = 42) -> List[Dict[str, Any]]:
    """Generates synthetic network telemetry records mimicking latency, jitter, and packet loss."""
    np.random.seed(seed)
    base_rtt = 25.0
    records = []
    
    rtt_state = base_rtt
    for i in range(n_samples):
        # Brownian random walk + burst congestion
        rtt_noise = np.random.normal(0, 1.5)
        burst = 35.0 if np.random.rand() < 0.03 else 0.0
        rtt_state = max(5.0, 0.95 * rtt_state + 0.05 * (base_rtt + rtt_noise) + burst)
        
        jitter = abs(np.random.normal(2.0, 1.0) + (burst * 0.4))
        loss = 0.01 if burst == 0 else np.random.uniform(0.05, 0.25)
        
        records.append({
            "sample_id": i,
            "rtt_ms": float(rtt_state),
            "jitter_ms": float(jitter),
            "loss_rate": float(loss),
            "timestamp": i * 0.01666
        })
    return records
