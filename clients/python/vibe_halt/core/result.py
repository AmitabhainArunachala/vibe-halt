"""Typed outcome surface for the strict Python-to-Rust adapter."""

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional


class Verdict(str, Enum):
    CLEAN = "CLEAN"
    FINDINGS = "FINDINGS"
    UNCHECKED = "UNCHECKED"
    ERROR = "ERROR"


class Tier(str, Enum):
    TIER1 = "TIER1"
    TIER2 = "TIER2"
    UNKNOWN = "UNKNOWN"


class Grade(str, Enum):
    D0 = "D0"
    D2 = "D2"
    UNTRUSTED = "UNTRUSTED"


@dataclass(frozen=True)
class Outcome:
    """Immutable engine-result data. Every field is present; nothing is
    manufactured by the Python side."""

    verdict: Verdict
    tier: Tier
    grade: Grade
    scope: str
    evidence_digest: Optional[str] = None
    invocation_envelope_digest: Optional[str] = None
    request_digest: Optional[str] = None
    receipt_dir: Optional[str] = None
    stdout: str = ""
    stderr: str = ""
    exit_code: int = -1
    verified: bool = False
    errors: List[str] = field(default_factory=list)
    findings_count: int = 0
    clean_count: Optional[int] = None
    divergent_count: Optional[int] = None
    raw: Dict[str, Any] = field(default_factory=dict, repr=False)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "verdict": self.verdict.value,
            "tier": self.tier.value,
            "grade": self.grade.value,
            "scope": self.scope,
            "evidence_digest": self.evidence_digest,
            "invocation_envelope_digest": self.invocation_envelope_digest,
            "request_digest": self.request_digest,
            "receipt_dir": self.receipt_dir,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "exit_code": self.exit_code,
            "verified": self.verified,
            "errors": list(self.errors),
            "findings_count": self.findings_count,
            "clean_count": self.clean_count,
            "divergent_count": self.divergent_count,
        }
