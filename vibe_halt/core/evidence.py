from dataclasses import dataclass, field
from typing import Any, Dict, List


@dataclass
class EvidenceReport:
    """Rich evidence output from a multiverse run."""

    universes_run: int
    violations: List[Dict[str, Any]] = field(default_factory=list)
    reproducibility_score: float = 1.0
    summary: str = ""

    def __post_init__(self):
        if not self.summary:
            if self.violations:
                self.summary = f"{len(self.violations)} violations across {self.universes_run} universes"
            else:
                self.summary = f"All properties held across {self.universes_run} universes"

    def to_dict(self) -> Dict[str, Any]:
        return {
            "universes_run": self.universes_run,
            "violations": self.violations,
            "reproducibility_score": self.reproducibility_score,
            "summary": self.summary,
        }
