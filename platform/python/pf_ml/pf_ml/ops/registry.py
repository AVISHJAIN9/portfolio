import time
from typing import Any, Dict, Optional

class ModelRegistry:
    """Manages model versions, staged promotions, and instant rollbacks (<5 minutes SLA)."""
    def __init__(self):
        self.models: Dict[str, Dict[str, Any]] = {}
        self.active_versions: Dict[str, str] = {}

    def register(self, model_name: str, version: str, artifact: Any, metadata: Optional[Dict] = None):
        if model_name not in self.models:
            self.models[model_name] = {}
        self.models[model_name][version] = {
            "artifact": artifact,
            "registered_at": time.time(),
            "metadata": metadata or {}
        }
        self.active_versions[model_name] = version

    def rollback(self, model_name: str, version: str) -> bool:
        if model_name in self.models and version in self.models[model_name]:
            self.active_versions[model_name] = version
            return True
        return False
