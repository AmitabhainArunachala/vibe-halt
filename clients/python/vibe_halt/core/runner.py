from typing import Any, Dict, List

from .rng import SeededRNG
from .faults import FaultInjector
from .properties import PROPERTIES
from .sandbox import Sandbox
from .evidence import EvidenceReport


class MultiverseRunner:
    """Core multiverse runner for the Mega Hyper Vibration Multiverse Halting Machine."""

    def __init__(self, target: str, universes: int = 1000, base_seed: int = 42):
        self.target = target
        self.universes = universes
        self.base_seed = base_seed
        self.rng = SeededRNG(base_seed)
        self.fault_injector = FaultInjector()
        self.sandbox = Sandbox()

    def run(self) -> EvidenceReport:
        violations: List[Dict] = []
        for i in range(self.universes):
            seed = self.base_seed + i
            # Simulate one universe
            result = self._run_universe(seed)
            for name, prop in PROPERTIES.items():
                if not prop.check(result):
                    violations.append({"universe": i, "property": name})

        return EvidenceReport(
            universes_run=self.universes,
            violations=violations,
            reproducibility_score=1.0 if not violations else 0.8,
        )

    def _run_universe(self, seed: int) -> Dict[str, Any]:
        # Stub universe execution
        self.rng = SeededRNG(seed)  # reset for reproducibility
        # Example: inject a fault sometimes
        if self.rng.random() < 0.1:
            self.fault_injector.inject("hallucinated_import", "fake_module")
        return {
            "seed": seed,
            "exceptions": [],
            "state_coherent": True,
        }
