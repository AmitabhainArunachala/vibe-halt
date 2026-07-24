"""Evidence schema for the (quarantined) Python client.

C3-honesty (controller section 7; pre-repair defect at evidence.py:11,19
of dfc0551): this dataclass used to manufacture evidence — a publicly
constructible report defaulted to reproducibility_score=1.0 and, when no
summary was supplied, minted "All properties held across N universes"
without any runner-owned evidence. Every field is now caller-required
and perfection claims fail closed: the Python execution path is
quarantined (see runner.QUARANTINE_MESSAGE), so no public construction
can earn them. A runner-evidence path may reopen this only when the
strict Rust-engine client lands (Phase 4).
"""

import math
from dataclasses import dataclass
from typing import Any, Dict, List


class ManufacturedEvidenceError(ValueError):
    """A report claimed evidence that no runner supplied."""


@dataclass
class EvidenceReport:
    """Evidence output from a multiverse run. Every field is
    caller-supplied; nothing is defaulted or derived."""

    universes_run: int
    violations: List[Dict[str, Any]]
    reproducibility_score: float
    summary: str

    def __post_init__(self):
        if not isinstance(self.summary, str) or not self.summary:
            raise ValueError("summary must be a non-empty string; it is never manufactured")
        if "all properties held" in self.summary.lower():
            raise ManufacturedEvidenceError(
                '"All properties held" is a runner-owned verdict; the Python '
                "execution path is quarantined and supplies no evidence for it"
            )
        score = self.reproducibility_score
        if (
            isinstance(score, bool)
            or not isinstance(score, (int, float))
            or not math.isfinite(score)
        ):
            raise ValueError(f"reproducibility_score must be a finite number, got {score!r}")
        if score == 1.0:
            raise ManufacturedEvidenceError(
                "reproducibility_score=1.0 is a runner-owned claim; the Python "
                "execution path is quarantined and supplies no evidence for it"
            )
        if not 0.0 <= score < 1.0:
            raise ValueError(f"reproducibility_score must be in [0.0, 1.0), got {score!r}")

    def to_dict(self) -> Dict[str, Any]:
        return {
            "universes_run": self.universes_run,
            "violations": self.violations,
            "reproducibility_score": self.reproducibility_score,
            "summary": self.summary,
        }
