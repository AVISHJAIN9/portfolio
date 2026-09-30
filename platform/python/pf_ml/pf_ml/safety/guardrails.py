class SafetyGuardrails:
    """Enforces hard bounded safety envelopes, confidence gating, and deterministic kill-switch fallbacks."""
    def __init__(self, min_val: float, max_val: float, default_val: float, min_confidence: float = 0.70):
        self.min_val = min_val
        self.max_val = max_val
        self.default_val = default_val
        self.min_confidence = min_confidence
        self.kill_switch_active = False

    def trigger_kill_switch(self):
        self.kill_switch_active = True

    def reset_kill_switch(self):
        self.kill_switch_active = False

    def sanitize(self, proposed_val: float, confidence: float = 1.0) -> float:
        if self.kill_switch_active or proposed_val is None or confidence < self.min_confidence:
            return self.default_val
        return max(self.min_val, min(self.max_val, float(proposed_val)))
