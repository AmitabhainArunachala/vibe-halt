"""Typed outcome surface for the strict Python-to-Rust adapter."""

from dataclasses import InitVar, dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional, Tuple, final


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
class ProtocolReport:
    """Read-only data mapped from a validated Rust negotiation record."""

    manifest_id: str
    operation: str
    features: Tuple[str, ...]
    engine_request_id: str
    receipt_schema: str
    verify_schema: str


@final
@dataclass(frozen=True)
class RevisionReport:
    """Mapped v2 data or explicit legacy classification, never authority.

    In particular, callers cannot construct a value that looks like a fresh
    or verified revision observation.  Only the strict runner mapping path can
    instantiate this read-only report.
    """

    observation_subject: Optional[str]
    revision_algorithm: Optional[str]
    revision_policy: Optional[str]
    requested: Optional[str]
    claimed_observed: Optional[str]
    fresh_observed: Optional[str]
    verified_observed: Optional[str]
    binding: str
    execution_binding: str
    observation_to_exec_channel: str

    _factory_token: InitVar[object] = None

    def __post_init__(self, _factory_token: object) -> None:
        authority_shaped = (
            self.binding == "bound"
            or self.fresh_observed is not None
            or self.verified_observed is not None
        )
        if authority_shaped and not _has_result_factory_authority(_factory_token):
            raise TypeError("bound RevisionReport instances are runner-produced")

    def __init_subclass__(cls, **kwargs: Any) -> None:
        raise TypeError("RevisionReport cannot be subclassed")


@dataclass(frozen=True)
class RefusalReport:
    """Strictly parsed same-engine refusal data; never a positive verdict."""

    reason: str
    executions: int
    manifest_id: str


@final
@dataclass(frozen=True)
class Outcome:
    """Frozen top-level caller-process data mapped from validated Rust records.

    Python also constructs request/envelope digests and downgrade diagnostics;
    this object is therefore not an authority or same-process trust boundary.
    `verified` is trust-qualified: it is true only after authentic Rust replay
    by an engine whose SHA-256 matches `EnginePolicy.expected_digest`. The raw
    record may retain the engine's narrower mechanical verification fields.
    """

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
    protocol: Optional[ProtocolReport] = None
    revision: Optional[RevisionReport] = None
    refusal: Optional[RefusalReport] = None
    _factory_token: InitVar[object] = None

    def __post_init__(self, _factory_token: object) -> None:
        # Preserve the historical ability to create ordinary UNCHECKED/ERROR
        # data objects, but do not let a public constructor mint a checked or
        # verified claim.  Positive results are produced only by the runner's
        # private mapping factory after strict Rust replay validation.
        authority_shaped = (
            self.grade is not Grade.UNTRUSTED
            or self.verified
            or self.verdict in {Verdict.CLEAN, Verdict.FINDINGS}
        )
        if authority_shaped and not _has_result_factory_authority(_factory_token):
            raise TypeError("trust-qualified Outcome instances are runner-produced")

    def __init_subclass__(cls, **kwargs: Any) -> None:
        raise TypeError("Outcome cannot be subclassed")

    def to_dict(self) -> Dict[str, Any]:
        result = {
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
        if self.protocol is not None:
            result["protocol"] = {
                "manifest_id": self.protocol.manifest_id,
                "operation": self.protocol.operation,
                "features": list(self.protocol.features),
                "engine_request_id": self.protocol.engine_request_id,
                "receipt_schema": self.protocol.receipt_schema,
                "verify_schema": self.protocol.verify_schema,
            }
        if self.revision is not None:
            result["revision"] = {
                "observation_subject": self.revision.observation_subject,
                "revision_algorithm": self.revision.revision_algorithm,
                "revision_policy": self.revision.revision_policy,
                "requested": self.revision.requested,
                "claimed_observed": self.revision.claimed_observed,
                "fresh_observed": self.revision.fresh_observed,
                "verified_observed": self.revision.verified_observed,
                "binding": self.revision.binding,
                "execution_binding": self.revision.execution_binding,
                "observation_to_exec_channel": self.revision.observation_to_exec_channel,
            }
        if self.refusal is not None:
            result["refusal"] = {
                "reason": self.refusal.reason,
                "executions": self.refusal.executions,
                "manifest_id": self.refusal.manifest_id,
            }
        return result


def _build_result_factories():
    """Keep the supported-constructor capability out of module state.

    This hardens the public Python constructors; it is not an in-process
    memory-integrity boundary against ``object.__new__``, pickle abuse, or
    callers deliberately invoking underscore-prefixed internals.  Rust
    remains the evidence authority.
    """

    capability = object()

    def has_authority(candidate: object) -> bool:
        return candidate is capability

    def make_revision_report(**values: Any) -> RevisionReport:
        return RevisionReport(_factory_token=capability, **values)

    def make_outcome(**values: Any) -> Outcome:
        return Outcome(_factory_token=capability, **values)

    return has_authority, make_revision_report, make_outcome


(
    _has_result_factory_authority,
    _make_revision_report,
    _make_outcome,
) = _build_result_factories()
del _build_result_factories
