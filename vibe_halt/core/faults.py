from typing import Callable, Dict, List


class FaultInjector:
    """Registry of gremlins for vibe-coded code stress testing."""

    def __init__(self):
        self._gremlins: Dict[str, Callable] = {}
        self._register_default_gremlins()

    def _register_default_gremlins(self):
        self.register("hallucinated_import", self._hallucinated_import)
        self.register("state_corruption", self._state_corruption)
        self.register("swallowed_exception", self._swallowed_exception)

    def register(self, name: str, func: Callable):
        self._gremlins[name] = func

    def inject(self, name: str, *args, **kwargs):
        if name in self._gremlins:
            return self._gremlins[name](*args, **kwargs)
        return None

    # Example gremlins (expand in Phase 1+)
    def _hallucinated_import(self, module_name: str):
        # Simulate hallucinated import
        print(f"[GREMLIN] Hallucinated import: {module_name}")
        return f"# Hallucinated: import {module_name}"

    def _state_corruption(self, state: dict):
        if state:
            key = list(state.keys())[0]
            state[key] = "CORRUPTED_BY_VIBE"
        return state

    def _swallowed_exception(self):
        print("[GREMLIN] Swallowed exception triggered")
        return None
