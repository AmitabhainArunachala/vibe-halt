"""Strict Python-to-Rust adapter: one runner method + reverify."""

from __future__ import annotations

import hashlib
import json
import os
import secrets
import stat
import subprocess
import sys
import tempfile
import unicodedata
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .request import (
    EnginePolicy,
    FeatureId,
    OperationId,
    ProtocolRequirement,
    RequestedTargetRevision,
    RunRequest,
)
from .result import (
    Grade,
    Outcome as OutcomeRecord,
    ProtocolReport,
    RefusalReport,
    RevisionReport as RevisionReportRecord,
    Tier,
    Verdict,
    _make_outcome,
    _make_revision_report,
)

# Keep every supported runner mapping behind result.py's private capability.
# The public classes remain return types, never trust-positive constructors.
Outcome = _make_outcome
RevisionReport = _make_revision_report


SCOPE = "vibe-halt.run.v0"
VERIFY_RUN_SCHEMA = "vh-verify-run-v2"
VERIFY_COOPERATIVE_SCHEMA = "vh-cooperative-verify-v1"
COOPERATIVE_RECEIPT_NAME = "cooperative.receipt"
RUN_RECEIPT_NAME = "run.ndjson"
MAX_ENGINE_BYTES = 128 << 20
MAX_ENGINE_OUTPUT_BYTES = 1 << 20
MAX_COOPERATIVE_RECEIPT_BYTES = 4 << 20
ENGINE_INVOCATION_TIMEOUT_SECONDS = 120
MAX_DIAGNOSTIC_BYTES = 256
MAX_DIAGNOSTIC_ITEMS = 64
_CANONICAL_JSON_SEPARATORS = (",", ":")
_GENERIC_ENGINE_REQUEST_DOMAIN = "vh-generic-engine-request-v1"
PROTOCOL_MANIFEST_SCHEMA = "vh-protocol-manifest-v1"
ENGINE_REFUSAL_SCHEMA = "vh-engine-negotiation-refusal-v1"
COOPERATIVE_OPERATION_V1 = "cooperative-target-v1"
COOPERATIVE_REQUEST_SCHEMA_V2 = "vh-cooperative-request-v2"
COOPERATIVE_OUTCOME_SCHEMA_V2 = "vh-cooperative-outcome-v2"
COOPERATIVE_RECEIPT_SCHEMA_V2 = "vh-cooperative-receipt-v2"
COOPERATIVE_VERIFY_SCHEMA_V2 = "vh-cooperative-verify-v2"
COOPERATIVE_VERIFY_FAILURE_SCHEMA_V1 = "vh-cooperative-verify-failure-v1"
COOPERATIVE_OBSERVATION_SUBJECT_V1 = "cooperative-child-source-v1"
COOPERATIVE_REVISION_ALGORITHM = "sha256"
COOPERATIVE_REVISION_POLICY = "bound-required"
COOPERATIVE_EXECUTION_BINDING = "staged-d2"
COOPERATIVE_OBSERVATION_TO_EXEC_CHANNEL = "open"
COOPERATIVE_MANDATORY_FEATURES = (
    "cooperative-cassette-v2",
    "fresh-replay-v1",
    "observed-child-source-sha256-v1",
)
MAX_PROTOCOL_RECORD_BYTES = 64 << 10
MAX_PROTOCOL_FEATURES = 16
MAX_PROTOCOL_IDENTIFIER_BYTES = 64
_PROTOCOL_MANIFEST_ID_DOMAIN = "vh-protocol-manifest-id-v1"
_REFUSAL_REASONS = {
    "unsupported-operation",
    "unsupported-feature",
    "invalid-feature-set",
    "protocol-manifest-mismatch",
    "requested-revision-mismatch",
    "missing-observation",
    "unsupported-receipt-schema",
}
_VERIFY_FAILURE_REASONS = {
    "malformed-receipt",
    "expected-request-mismatch",
    "revision-mismatch",
    "identity-mismatch",
    "fresh-replay-failed",
}


@dataclass(frozen=True)
class _ProtocolDescriptor:
    operation: str
    request_schema: str
    outcome_schema: str
    receipt_schema: str
    verifier_schema: str
    observation_subject: str
    revision_algorithm: str
    revision_policy: str
    execution_binding: str
    observation_to_exec_channel: str
    mandatory_features: Tuple[str, ...]
    optional_features: Tuple[str, ...]


@dataclass(frozen=True)
class _ProtocolManifest:
    engine_sha256: str
    manifest_id: str
    descriptors: Tuple[_ProtocolDescriptor, ...]


@dataclass(frozen=True)
class _EngineRefusalRecord:
    reason: str
    engine_sha256: str
    manifest_id: str
    executions: int


@dataclass(frozen=True)
class _V2VerificationFailureRecord:
    reason: str
    engine_sha256: str
    manifest_id: str
    receipt_sha256: str
    executions: int
    authentic: bool
    verified: bool
    exit_code: int


def _canonical_json_bytes(obj: Any) -> bytes:
    return json.dumps(
        obj,
        ensure_ascii=False,
        separators=_CANONICAL_JSON_SEPARATORS,
        sort_keys=True,
        default=lambda o: asdict(o) if hasattr(o, "__dataclass_fields__") else str(o),
    ).encode("utf-8")


def _sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _snapshot_run_request(request: RunRequest) -> RunRequest:
    """Rebuild one exact immutable base request without nested ``asdict``.

    ``dataclasses.asdict`` converts nested frozen protocol types to ordinary
    dictionaries. Explicit reconstruction preserves exact-type validation at
    the hostile Python object boundary.
    """

    requirement = request.protocol_requirement
    if requirement is not None:
        if type(requirement) is not ProtocolRequirement:
            raise TypeError("protocol_requirement must be a ProtocolRequirement")
        if type(requirement.operation) is not OperationId:
            raise TypeError("protocol operation must be an OperationId")
        if type(requirement.required_features) is not tuple:
            raise TypeError("protocol required_features must be a tuple")
        if any(
            type(feature) is not FeatureId
            for feature in requirement.required_features
        ):
            raise TypeError("every protocol required feature must be a FeatureId")
        if (
            type(requirement.requested_target_revision)
            is not RequestedTargetRevision
        ):
            raise TypeError(
                "protocol requested revision must be a RequestedTargetRevision"
            )
        operation = requirement.operation
        revision = requirement.requested_target_revision
        requirement = ProtocolRequirement(
            operation=OperationId(operation.name, operation.version),
            required_features=tuple(
                FeatureId(feature.name, feature.version)
                for feature in requirement.required_features
            ),
            requested_target_revision=RequestedTargetRevision(
                revision.subject, revision.algorithm, revision.digest
            ),
        )
    return RunRequest(
        workload=request.workload,
        universes=request.universes,
        seed=request.seed,
        palette=request.palette,
        schedule=request.schedule,
        check_divergence=request.check_divergence,
        record_tape=request.record_tape,
        shrink=request.shrink,
        source_commit=request.source_commit,
        output_root=request.output_root,
        invocation_id=request.invocation_id,
        transport=request.transport,
        cassette_path=request.cassette_path,
        protocol_requirement=requirement,
    )


def _request_dict(request: RunRequest) -> Dict[str, Any]:
    """Canonical client correlation request, preserving legacy preimages."""

    value: Dict[str, Any] = {
        "workload": request.workload,
        "universes": request.universes,
        "seed": request.seed,
        "palette": request.palette,
        "schedule": request.schedule,
        "check_divergence": request.check_divergence,
        "record_tape": request.record_tape,
        "shrink": request.shrink,
        "source_commit": request.source_commit,
        "transport": request.transport,
        "cassette_path": request.cassette_path,
    }
    requirement = request.protocol_requirement
    if requirement is not None:
        value["protocol_requirement"] = {
            "operation": requirement.operation.value,
            "required_features": [
                feature.value for feature in requirement.required_features
            ],
            "requested_target_revision": {
                "subject": requirement.requested_target_revision.subject,
                "algorithm": requirement.requested_target_revision.algorithm,
                "digest": requirement.requested_target_revision.digest,
            },
        }
    return value


def _generic_engine_request_digest(request: RunRequest) -> str:
    """Mirror Rust's domain-separated semantic receipt request binding."""

    framed = bytearray((_GENERIC_ENGINE_REQUEST_DOMAIN + "\n").encode("ascii"))

    def add(tag: str, value: bytes) -> None:
        framed.extend(tag.encode("ascii"))
        framed.extend(b" ")
        framed.extend(str(len(value)).encode("ascii"))
        framed.extend(b":")
        framed.extend(value)
        framed.extend(b"\n")

    add("workload", request.workload.encode("utf-8"))
    add("seed", f"0x{request.seed:x}".encode("ascii"))
    add("universes", str(request.universes).encode("ascii"))
    add("palette", request.palette.encode("utf-8"))
    add("divergence-check", b"true" if request.check_divergence else b"false")
    add("schedule", b"fifo")
    add("record-tape", b"false")
    if request.source_commit is None:
        add("source-commit-present", b"false")
    else:
        add("source-commit-present", b"true")
        add("source-commit", request.source_commit.encode("utf-8"))
    return _sha256_hex(bytes(framed))


def _copy_and_verify_engine(
    policy: EnginePolicy, private_dir: Path
) -> Tuple[Path, bool, str]:
    """Copy the engine into `private_dir` and verify it when a trust root exists.

    Returns the path to run and a flag that is True when no trust root was
    configured."""
    source = Path(policy.path)
    try:
        if not os.path.lexists(source):
            raise ValueError("engine path does not exist")
        _reject_symlink_components(source)
        flags = os.O_RDONLY
        flags |= getattr(os, "O_NOFOLLOW", 0)
        flags |= getattr(os, "O_NONBLOCK", 0)
        descriptor = os.open(source, flags)
        try:
            metadata = os.fstat(descriptor)
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError("engine path is not a regular no-link file")
            if metadata.st_size > MAX_ENGINE_BYTES:
                raise ValueError(
                    f"engine exceeds the {MAX_ENGINE_BYTES}-byte snapshot bound"
                )
            with os.fdopen(descriptor, "rb", closefd=True) as handle:
                descriptor = -1
                engine_bytes = handle.read(MAX_ENGINE_BYTES + 1)
            if len(engine_bytes) > MAX_ENGINE_BYTES:
                raise ValueError(
                    f"engine exceeds the {MAX_ENGINE_BYTES}-byte snapshot bound"
                )
        finally:
            if descriptor >= 0:
                os.close(descriptor)
    except FileNotFoundError:
        raise ValueError("engine path does not exist") from None
    except ValueError:
        raise
    except OSError as exc:
        raise ValueError(
            f"engine snapshot refused: {exc.strerror or exc.__class__.__name__}"
        ) from None

    actual = _sha256_hex(engine_bytes)
    untrusted = policy.expected_digest is None
    if not untrusted and actual != policy.expected_digest:
        raise ValueError(
            f"engine digest mismatch: expected {policy.expected_digest}, got {actual}"
        )
    dest = private_dir / ".vibe-halt-engine"
    try:
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
        flags |= getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(dest, flags, 0o700)
        try:
            os.fchmod(descriptor, 0o700)
            with os.fdopen(descriptor, "wb", closefd=True) as handle:
                descriptor = -1
                handle.write(engine_bytes)
                handle.flush()
                os.fsync(handle.fileno())
        finally:
            if descriptor >= 0:
                os.close(descriptor)
    except OSError as exc:
        raise ValueError(
            f"engine snapshot publication failed: {exc.strerror or exc.__class__.__name__}"
        ) from None
    # Re-open the published private copy rather than carrying the source
    # observation across publication. This digest binds manifest consistency
    # to the pathname that every later command actually invokes. It remains a
    # D2 observation, not a closed observation-to-exec handoff.
    try:
        _reject_symlink_components(dest)
        flags = os.O_RDONLY
        flags |= getattr(os, "O_NOFOLLOW", 0)
        flags |= getattr(os, "O_NONBLOCK", 0)
        descriptor = os.open(dest, flags)
        try:
            metadata = os.fstat(descriptor)
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAX_ENGINE_BYTES:
                raise ValueError("published engine copy is not a bounded regular file")
            with os.fdopen(descriptor, "rb", closefd=True) as handle:
                descriptor = -1
                copied_bytes = handle.read(MAX_ENGINE_BYTES + 1)
            if len(copied_bytes) > MAX_ENGINE_BYTES or copied_bytes != engine_bytes:
                raise ValueError("published engine copy changed during re-observation")
        finally:
            if descriptor >= 0:
                os.close(descriptor)
    except ValueError:
        raise
    except OSError as exc:
        raise ValueError(
            f"published engine re-observation failed: {exc.strerror or exc.__class__.__name__}"
        ) from None
    return dest, untrusted, _sha256_hex(copied_bytes)


def _read_bounded_process_stream(handle) -> Tuple[str, bool]:
    handle.seek(0)
    data = handle.read(MAX_ENGINE_OUTPUT_BYTES + 1)
    truncated = len(data) > MAX_ENGINE_OUTPUT_BYTES
    data = data[:MAX_ENGINE_OUTPUT_BYTES]
    text = data.decode("utf-8", errors="replace")
    return text, truncated


def _read_bounded_process_bytes(handle) -> Tuple[bytes, bool]:
    handle.seek(0)
    data = handle.read(MAX_ENGINE_OUTPUT_BYTES + 1)
    return data[:MAX_ENGINE_OUTPUT_BYTES], len(data) > MAX_ENGINE_OUTPUT_BYTES


def _invoke_engine_bytes(
    argv: List[str], cwd: Optional[Path] = None
) -> subprocess.CompletedProcess[bytes]:
    """No-shell invocation retaining exact bytes for strict wire parsers."""

    with tempfile.TemporaryFile(mode="w+b") as stdout_file, tempfile.TemporaryFile(
        mode="w+b"
    ) as stderr_file:
        try:
            completed = subprocess.run(
                argv,
                cwd=cwd,
                stdout=stdout_file,
                stderr=stderr_file,
                text=False,
                shell=False,
                env={"PATH": os.environ.get("PATH", "")},
                timeout=ENGINE_INVOCATION_TIMEOUT_SECONDS,
                check=False,
            )
            stdout, stdout_truncated = _read_bounded_process_bytes(stdout_file)
            stderr, stderr_truncated = _read_bounded_process_bytes(stderr_file)
            if stdout_truncated or stderr_truncated:
                return subprocess.CompletedProcess(
                    argv,
                    125,
                    stdout,
                    stderr + b"\nengine output exceeded its bounded capture",
                )
            return subprocess.CompletedProcess(argv, completed.returncode, stdout, stderr)
        except subprocess.TimeoutExpired:
            stdout, _ = _read_bounded_process_bytes(stdout_file)
            return subprocess.CompletedProcess(
                argv,
                124,
                stdout,
                b"engine invocation exceeded its bounded deadline",
            )
        except (OSError, ValueError) as exc:
            stdout, _ = _read_bounded_process_bytes(stdout_file)
            return subprocess.CompletedProcess(
                argv,
                126,
                stdout,
                f"engine invocation refused: {exc.__class__.__name__}".encode("ascii"),
            )


def _invoke_engine(
    argv: List[str], cwd: Optional[Path] = None
) -> subprocess.CompletedProcess[str]:
    """Invoke with a configured timeout and bounded retained output.

    The direct-child timeout fails closed, but Python's post-kill wait and
    temporary capture-file growth are not separately hard-bounded here.
    """
    with tempfile.TemporaryFile(mode="w+b") as stdout_file, tempfile.TemporaryFile(
        mode="w+b"
    ) as stderr_file:
        try:
            completed = subprocess.run(
                argv,
                cwd=cwd,
                stdout=stdout_file,
                stderr=stderr_file,
                text=False,
                shell=False,
                env={"PATH": os.environ.get("PATH", "")},
                timeout=ENGINE_INVOCATION_TIMEOUT_SECONDS,
                check=False,
            )
            stdout, stdout_truncated = _read_bounded_process_stream(stdout_file)
            stderr, stderr_truncated = _read_bounded_process_stream(stderr_file)
            if stdout_truncated or stderr_truncated:
                return subprocess.CompletedProcess(
                    argv,
                    125,
                    stdout,
                    stderr + "\nengine output exceeded its bounded capture",
                )
            return subprocess.CompletedProcess(argv, completed.returncode, stdout, stderr)
        except subprocess.TimeoutExpired:
            stdout, _ = _read_bounded_process_stream(stdout_file)
            return subprocess.CompletedProcess(
                argv,
                124,
                stdout,
                "engine invocation exceeded its bounded deadline",
            )
        except (OSError, ValueError) as exc:
            stdout, _ = _read_bounded_process_stream(stdout_file)
            return subprocess.CompletedProcess(
                argv,
                126,
                stdout,
                f"engine invocation refused: {exc.__class__.__name__}",
            )


def _reject_symlink_components(path: Path, *, allow_missing_leaf: bool = False) -> None:
    """Reject traversal and every observable symlink component.

    This is a fail-closed preflight, not a claim that Python closes the
    same-user observation-to-use channel. The Rust boundary independently
    repeats admission with no-follow file opens.
    """
    if not path.is_absolute():
        raise ValueError("path must be absolute")
    if any(part in {".", ".."} for part in path.parts):
        raise ValueError("path traversal components are not allowed")
    current = Path(path.anchor)
    tail = path.parts[1:] if path.anchor else path.parts
    for index, part in enumerate(tail):
        current /= part
        try:
            mode = os.lstat(current).st_mode
        except FileNotFoundError:
            if allow_missing_leaf and index == len(tail) - 1:
                return
            raise ValueError("path has a missing parent component") from None
        except OSError as exc:
            raise ValueError(
                f"path inspection failed: {exc.strerror or exc.__class__.__name__}"
            ) from None
        trusted_macos_alias = sys.platform == "darwin" and str(current) in {
            "/tmp",
            "/var",
        }
        if stat.S_ISLNK(mode) and not trusted_macos_alias:
            raise ValueError("path contains a symlink component")


def _validate_cross_uid_safe_directory(
    path: Path, *, require_private_leaf: bool = False
) -> None:
    """Validate a directory chain used by trusted pathname operations.

    On Unix, every real component must be owned by root or this process.
    Group/other-writable components are accepted only when sticky and
    root-owned; a sticky directory owned by another user can still replace
    child entries. ACL and same-UID races remain outside this D2 boundary.
    """
    _reject_symlink_components(path)
    effective_uid = os.geteuid() if hasattr(os, "geteuid") else None
    if effective_uid is None:
        raise ValueError("cross-uid directory ownership cannot be proven")
    current = Path(path.anchor)
    tail = path.parts[1:] if path.anchor else path.parts
    observed_paths = [current] if path.anchor else []
    for part in tail:
        current /= part
        observed_paths.append(current)
    leaf_metadata = None
    try:
        for current in observed_paths:
            metadata = os.lstat(current)
            trusted_macos_alias = sys.platform == "darwin" and str(current) in {
                "/tmp",
                "/var",
            }
            if stat.S_ISLNK(metadata.st_mode) and trusted_macos_alias:
                continue
            if not stat.S_ISDIR(metadata.st_mode):
                raise ValueError("trusted directory chain contains a non-directory")
            if metadata.st_uid not in {0, effective_uid}:
                raise ValueError("trusted directory chain has an untrusted owner")
            mode = stat.S_IMODE(metadata.st_mode)
            if mode & 0o022 and not (mode & stat.S_ISVTX and metadata.st_uid == 0):
                raise ValueError("trusted directory chain has an unsafe shared parent")
            leaf_metadata = metadata
    except ValueError:
        raise
    except OSError as exc:
        raise ValueError(
            f"trusted directory inspection failed: {exc.strerror or exc.__class__.__name__}"
        ) from None
    if leaf_metadata is None:
        leaf_metadata = os.lstat(path)
    if require_private_leaf:
        leaf_mode = stat.S_IMODE(leaf_metadata.st_mode)
        if leaf_metadata.st_uid != effective_uid or leaf_mode & 0o777 != 0o700:
            raise ValueError("private directory is not process-owned mode 0700")


def _trusted_temp_base() -> Path:
    base = Path(tempfile.gettempdir())
    _validate_cross_uid_safe_directory(base)
    return base


def _private_engine_directory(prefix: str):
    """Create and reobserve a private engine-directory lease."""
    base = _trusted_temp_base()
    lease = tempfile.TemporaryDirectory(
        prefix=prefix,
        dir=str(base),
        ignore_cleanup_errors=True,
    )
    private_dir = Path(lease.name)
    try:
        _validate_cross_uid_safe_directory(private_dir, require_private_leaf=True)
    except (OSError, ValueError):
        lease.cleanup()
        raise
    return lease, private_dir


def _regular_file_without_links(path: Path) -> bool:
    try:
        _reject_symlink_components(path)
        return stat.S_ISREG(os.lstat(path).st_mode)
    except (OSError, ValueError):
        return False


def _read_bounded_regular_file(path: Path, maximum: int) -> bytes:
    """Reobserve one bounded regular pathname without following its leaf."""

    try:
        _reject_symlink_components(path)
        flags = os.O_RDONLY
        flags |= getattr(os, "O_NOFOLLOW", 0)
        flags |= getattr(os, "O_NONBLOCK", 0)
        descriptor = os.open(path, flags)
        try:
            metadata = os.fstat(descriptor)
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError("path is not a regular no-link file")
            if metadata.st_size > maximum:
                raise ValueError("file exceeds its bounded read profile")
            with os.fdopen(descriptor, "rb", closefd=True) as handle:
                descriptor = -1
                value = handle.read(maximum + 1)
            if len(value) > maximum:
                raise ValueError("file exceeds its bounded read profile")
            return value
        finally:
            if descriptor >= 0:
                os.close(descriptor)
    except ValueError:
        raise
    except OSError as exc:
        raise ValueError(
            f"bounded file read refused: {exc.strerror or exc.__class__.__name__}"
        ) from None


def _prepare_output_root(request: RunRequest) -> Path:
    if request.output_root is not None:
        raw = Path(request.output_root)
        try:
            _reject_symlink_components(raw, allow_missing_leaf=True)
            _validate_cross_uid_safe_directory(raw.parent)
            try:
                os.lstat(raw)
            except FileNotFoundError:
                pass
            else:
                raise ValueError(
                    "output root already exists; caller-supplied roots must be absent"
                )
            # Exclusive creation is the reservation. An empty pre-existing
            # directory is not reusable: another actor may retain authority
            # over its pathname or contents.
            os.mkdir(raw, mode=0o700)
            _validate_cross_uid_safe_directory(raw, require_private_leaf=True)
        except ValueError:
            raise
        except OSError as exc:
            raise ValueError(
                f"output root refused: {exc.strerror or exc.__class__.__name__}"
            ) from None
        return raw
    generated = Path(
        tempfile.mkdtemp(prefix="vibe-halt-run-", dir=str(_trusted_temp_base()))
    )
    try:
        _validate_cross_uid_safe_directory(generated, require_private_leaf=True)
    except (OSError, ValueError):
        try:
            os.rmdir(generated)
        except OSError:
            pass
        raise
    return generated


def _build_run_args(request: RunRequest, out_dir: Path) -> List[str]:
    args: List[str] = [
        "run",
        "--workload",
        request.workload,
        "--seed",
        f"0x{request.seed:x}",
        "--universes",
        str(request.universes),
        "--palette",
        request.palette,
        "--schedule",
        request.schedule,
        "--out",
        str(out_dir),
    ]
    if not request.check_divergence:
        args.append("--no-divergence-check")
    if request.record_tape:
        args.append("--record-tape")
    if request.shrink:
        args.append("--shrink")
    if request.source_commit is not None:
        args.extend(["--source-commit", request.source_commit])
    return args


def _wire_frame(tag: str, value: bytes) -> bytes:
    return tag.encode("ascii") + b" " + str(len(value)).encode("ascii") + b":" + value + b"\n"


class _FramedRecordReader:
    """Total reader for Rust's canonical positional protocol records."""

    def __init__(self, data: bytes, maximum: int = MAX_PROTOCOL_RECORD_BYTES):
        if type(data) is not bytes:
            raise TypeError("protocol record must be bytes")
        if not data or len(data) > maximum:
            raise ValueError("protocol record is empty or oversized")
        self.data = data
        self.pos = 0

    def line(self) -> bytes:
        newline = self.data.find(b"\n", self.pos)
        if newline < 0:
            raise ValueError("truncated protocol line")
        value = self.data[self.pos:newline]
        if not value:
            raise ValueError("blank protocol line")
        self.pos = newline + 1
        return value

    def exact(self, expected: str) -> None:
        if self.line() != expected.encode("ascii"):
            raise ValueError("protocol field order mismatch")

    def count(self, tag: str, maximum: int) -> int:
        prefix = tag.encode("ascii") + b" "
        line = self.line()
        if not line.startswith(prefix):
            raise ValueError("protocol count field order mismatch")
        raw = line[len(prefix) :]
        if not raw or not raw.isdigit() or (len(raw) > 1 and raw.startswith(b"0")):
            raise ValueError("noncanonical protocol count")
        value = int(raw)
        if value > maximum:
            raise ValueError("protocol count exceeds its bound")
        return value

    def boolean(self, tag: str) -> bool:
        line = self.line()
        if line == f"{tag} true".encode("ascii"):
            return True
        if line == f"{tag} false".encode("ascii"):
            return False
        raise ValueError("noncanonical protocol boolean")

    def framed(self, tag: str, maximum: int = MAX_PROTOCOL_IDENTIFIER_BYTES) -> bytes:
        prefix = tag.encode("ascii") + b" "
        if not self.data.startswith(prefix, self.pos):
            raise ValueError("protocol framed field order mismatch")
        length_start = self.pos + len(prefix)
        colon = self.data.find(b":", length_start)
        if colon < 0:
            raise ValueError("truncated protocol frame")
        raw_length = self.data[length_start:colon]
        if (
            not raw_length
            or not raw_length.isdigit()
            or (len(raw_length) > 1 and raw_length.startswith(b"0"))
        ):
            raise ValueError("noncanonical protocol frame length")
        length = int(raw_length)
        if length > maximum:
            raise ValueError("protocol frame exceeds its bound")
        value_start = colon + 1
        value_end = value_start + length
        if value_end >= len(self.data) or self.data[value_end] != 0x0A:
            raise ValueError("truncated or malformed protocol frame")
        value = self.data[value_start:value_end]
        self.pos = value_end + 1
        return value

    def text(self, tag: str, maximum: int = MAX_PROTOCOL_IDENTIFIER_BYTES) -> str:
        try:
            return self.framed(tag, maximum).decode("ascii", errors="strict")
        except UnicodeDecodeError:
            raise ValueError("protocol text is not canonical ASCII") from None

    def finish(self) -> None:
        if self.pos != len(self.data):
            raise ValueError("trailing protocol data")


def _canonical_protocol_identifier(value: str) -> bool:
    return (
        0 < len(value) <= MAX_PROTOCOL_IDENTIFIER_BYTES
        and value[0] != "-"
        and value[-1] != "-"
        and all(character in "abcdefghijklmnopqrstuvwxyz0123456789-" for character in value)
    )


def _parse_protocol_manifest(data: bytes) -> _ProtocolManifest:
    reader = _FramedRecordReader(data)
    reader.exact(PROTOCOL_MANIFEST_SCHEMA)
    engine_sha256 = reader.text("engine-sha256")
    manifest_id = reader.text("manifest-id")
    if reader.count("descriptors", 1) != 1:
        raise ValueError("protocol manifest must contain exactly one descriptor")
    operation = reader.text("operation")
    request_schema = reader.text("request-schema")
    outcome_schema = reader.text("outcome-schema")
    receipt_schema = reader.text("receipt-schema")
    verifier_schema = reader.text("verifier-schema")
    observation_subject = reader.text("observation-subject")
    revision_algorithm = reader.text("revision-algorithm")
    revision_policy = reader.text("revision-policy")
    execution_binding = reader.text("execution-binding")
    observation_to_exec_channel = reader.text("observation-to-exec-channel")
    mandatory_count = reader.count("mandatory-features", MAX_PROTOCOL_FEATURES)
    mandatory_features = tuple(
        reader.text("feature") for _ in range(mandatory_count)
    )
    optional_count = reader.count("optional-features", MAX_PROTOCOL_FEATURES)
    optional_features = tuple(reader.text("feature") for _ in range(optional_count))
    reader.finish()

    if not _is_lower_hex(engine_sha256, 64) or not _is_lower_hex(manifest_id, 64):
        raise ValueError("protocol manifest digest is not lowercase SHA-256")
    identifiers = (
        operation,
        request_schema,
        outcome_schema,
        receipt_schema,
        verifier_schema,
        observation_subject,
        revision_algorithm,
        revision_policy,
        execution_binding,
        observation_to_exec_channel,
        *mandatory_features,
        *optional_features,
    )
    if any(not _canonical_protocol_identifier(value) for value in identifiers):
        raise ValueError("protocol manifest contains a noncanonical identifier")
    all_features = mandatory_features + optional_features
    if (
        mandatory_features != tuple(sorted(mandatory_features))
        or optional_features != tuple(sorted(optional_features))
        or len(set(all_features)) != len(all_features)
    ):
        raise ValueError("protocol manifest features are not sorted and unique")

    preimage = bytearray((_PROTOCOL_MANIFEST_ID_DOMAIN + "\n").encode("ascii"))
    for tag, value in (
        ("schema", PROTOCOL_MANIFEST_SCHEMA),
        ("engine-sha256", engine_sha256),
        ("operation", operation),
        ("request-schema", request_schema),
        ("outcome-schema", outcome_schema),
        ("receipt-schema", receipt_schema),
        ("verifier-schema", verifier_schema),
        ("observation-subject", observation_subject),
        ("revision-algorithm", revision_algorithm),
        ("revision-policy", revision_policy),
        ("execution-binding", execution_binding),
        ("observation-to-exec-channel", observation_to_exec_channel),
    ):
        preimage.extend(_wire_frame(tag, value.encode("ascii")))
    preimage.extend(f"mandatory-features {len(mandatory_features)}\n".encode("ascii"))
    for feature in mandatory_features:
        preimage.extend(_wire_frame("feature", feature.encode("ascii")))
    preimage.extend(f"optional-features {len(optional_features)}\n".encode("ascii"))
    for feature in optional_features:
        preimage.extend(_wire_frame("feature", feature.encode("ascii")))
    if _sha256_hex(bytes(preimage)) != manifest_id:
        raise ValueError("protocol manifest identity mismatch")

    # Manifest v1 has one closed Rust-owned descriptor. A recomputed digest
    # proves only self-consistency; it cannot authorize a new policy spelling,
    # causal execution claim, schema, feature set, or observation coordinate.
    expected_descriptor = (
        COOPERATIVE_OPERATION_V1,
        COOPERATIVE_REQUEST_SCHEMA_V2,
        COOPERATIVE_OUTCOME_SCHEMA_V2,
        COOPERATIVE_RECEIPT_SCHEMA_V2,
        COOPERATIVE_VERIFY_SCHEMA_V2,
        COOPERATIVE_OBSERVATION_SUBJECT_V1,
        COOPERATIVE_REVISION_ALGORITHM,
        COOPERATIVE_REVISION_POLICY,
        COOPERATIVE_EXECUTION_BINDING,
        COOPERATIVE_OBSERVATION_TO_EXEC_CHANNEL,
        COOPERATIVE_MANDATORY_FEATURES,
        (),
    )
    observed_descriptor = (
        operation,
        request_schema,
        outcome_schema,
        receipt_schema,
        verifier_schema,
        observation_subject,
        revision_algorithm,
        revision_policy,
        execution_binding,
        observation_to_exec_channel,
        mandatory_features,
        optional_features,
    )
    if observed_descriptor != expected_descriptor:
        raise ValueError("protocol manifest descriptor is unsupported by manifest v1")

    descriptor = _ProtocolDescriptor(
        operation=operation,
        request_schema=request_schema,
        outcome_schema=outcome_schema,
        receipt_schema=receipt_schema,
        verifier_schema=verifier_schema,
        observation_subject=observation_subject,
        revision_algorithm=revision_algorithm,
        revision_policy=revision_policy,
        execution_binding=execution_binding,
        observation_to_exec_channel=observation_to_exec_channel,
        mandatory_features=mandatory_features,
        optional_features=optional_features,
    )
    return _ProtocolManifest(engine_sha256, manifest_id, (descriptor,))


def _query_protocol_manifest(
    engine: Path, engine_dir: Path, copied_engine_digest: str
) -> _ProtocolManifest:
    process = _invoke_engine_bytes([str(engine), "protocol-manifest"], cwd=engine_dir)
    if process.returncode != 0 or process.stderr:
        raise ValueError("protocol manifest invocation failed")
    manifest = _parse_protocol_manifest(process.stdout)
    if manifest.engine_sha256 != copied_engine_digest:
        raise ValueError("protocol manifest engine digest differs from copied engine")
    return manifest


def _parse_engine_refusal(data: bytes) -> _EngineRefusalRecord:
    reader = _FramedRecordReader(data)
    reader.exact(ENGINE_REFUSAL_SCHEMA)
    reason = reader.text("reason")
    engine_sha256 = reader.text("engine-sha256")
    manifest_id = reader.text("manifest-id")
    executions = reader.count("executions", 0)
    reader.finish()
    if reason not in _REFUSAL_REASONS:
        raise ValueError("unknown engine refusal reason")
    if not _is_lower_hex(engine_sha256, 64) or not _is_lower_hex(manifest_id, 64):
        raise ValueError("engine refusal contains a malformed digest")
    return _EngineRefusalRecord(reason, engine_sha256, manifest_id, executions)


def _parse_v2_verification_failure(
    data: bytes,
) -> _V2VerificationFailureRecord:
    reader = _FramedRecordReader(data)
    reader.exact(COOPERATIVE_VERIFY_FAILURE_SCHEMA_V1)
    reason = reader.text("reason")
    engine_sha256 = reader.text("engine-sha256")
    manifest_id = reader.text("manifest-id")
    receipt_sha256 = reader.text("receipt-sha256")
    executions = reader.count("executions", 2)
    authentic = reader.boolean("authentic")
    verified = reader.boolean("verified")
    exit_code = reader.count("exit-code", 1)
    reader.finish()
    if reason not in _VERIFY_FAILURE_REASONS:
        raise ValueError("unknown v2 verification failure reason")
    if not _is_lower_hex(engine_sha256, 64) or not _is_lower_hex(manifest_id, 64):
        raise ValueError("v2 verification failure contains a malformed engine identity")
    if receipt_sha256 != "unavailable" and not _is_lower_hex(receipt_sha256, 64):
        raise ValueError("v2 verification failure contains a malformed receipt identity")
    if authentic or verified or exit_code != 1:
        raise ValueError("v2 verification failure has a positive or impossible shape")
    if reason == "fresh-replay-failed":
        if executions not in {1, 2}:
            raise ValueError("fresh replay failure lacks an admitted sandbox attempt")
    elif executions != 0:
        raise ValueError("pre-replay verification failure crossed the sandbox boundary")
    if receipt_sha256 == "unavailable" and reason != "malformed-receipt":
        raise ValueError("receipt identity is unavailable for a non-structural failure")
    return _V2VerificationFailureRecord(
        reason=reason,
        engine_sha256=engine_sha256,
        manifest_id=manifest_id,
        receipt_sha256=receipt_sha256,
        executions=executions,
        authentic=authentic,
        verified=verified,
        exit_code=exit_code,
    )


def _parse_v2_machine_record(data: bytes, expected_schema: str) -> Dict[str, Any]:
    reader = _FramedRecordReader(data)
    reader.exact(expected_schema)
    record: Dict[str, Any] = {
        "schema": expected_schema,
        "verdict": reader.text("verdict"),
        "tier": reader.text("tier"),
        "grade": reader.text("grade"),
        "scope": reader.text("scope"),
        "protocol_schema": reader.text("protocol-schema"),
        "manifest_id": reader.text("manifest-id"),
        "engine_sha256": reader.text("engine-sha256"),
        "operation": reader.text("operation"),
    }
    feature_count = reader.count("features", MAX_PROTOCOL_FEATURES)
    record["features_tuple"] = tuple(
        reader.text("feature") for _ in range(feature_count)
    )
    record.update(
        {
            "request_schema": reader.text("request-schema"),
            "outcome_schema": reader.text("outcome-schema"),
            "receipt_schema": reader.text("receipt-schema"),
            "verifier_schema": reader.text("verifier-schema"),
            "observation_subject": reader.text("observation-subject"),
            "revision_algorithm": reader.text("revision-algorithm"),
            "revision_policy": reader.text("revision-policy"),
            "requested_target_revision": reader.text(
                "requested-target-revision", 128
            ),
            "claimed_observed_revision": reader.text(
                "claimed-observed-revision"
            ),
            "fresh_observed_revision": reader.text("fresh-observed-revision"),
            "verified_observed_revision": reader.text(
                "verified-observed-revision"
            ),
            "revision_binding": reader.text("revision-binding"),
            "execution_binding": reader.text("execution-binding"),
            "observation_to_exec_channel": reader.text(
                "observation-to-exec-channel"
            ),
            "cassette_identity": reader.text("cassette-identity", 128),
            "engine_request_id": reader.text("engine-request-id"),
            "evidence_id": reader.text("evidence-id"),
            "result_digest": reader.text("result-digest"),
            "receipt_sha256": reader.text("receipt-sha256"),
            "verification_result_id": reader.text("verification-result-id"),
            "oracle": reader.text("oracle"),
            "oracle_evaluation": reader.text("oracle-evaluation"),
            "finding_identity": reader.text("finding-identity", 256),
            "findings_count": reader.count("findings-count", MAX_DIAGNOSTIC_ITEMS),
            "authentic": reader.boolean("authentic"),
            "verified": reader.boolean("verified"),
            "outcome_exit_code": reader.count("outcome-exit-code", 3),
            "exit_code": reader.count("exit-code", 4),
            "executions": reader.count("executions", 4),
        }
    )
    try:
        errors_json = reader.framed("errors", MAX_DIAGNOSTIC_ITEMS * (MAX_DIAGNOSTIC_BYTES + 8)).decode(
            "utf-8", errors="strict"
        )
    except UnicodeDecodeError:
        raise ValueError("v2 errors are not strict UTF-8") from None
    reader.finish()
    errors = _strict_errors(errors_json)
    if errors is None:
        raise ValueError("v2 errors are not canonical bounded diagnostics")
    record["errors"] = errors
    record["features"] = ",".join(record["features_tuple"])
    return record


def _validate_v2_machine_record(
    record: Dict[str, Any],
    *,
    expected_schema: str,
    process_returncode: int,
    manifest: _ProtocolManifest,
    descriptor: _ProtocolDescriptor,
    features: Tuple[str, ...],
    requested_revision: str,
) -> None:
    expected_executions = 4 if expected_schema == COOPERATIVE_OUTCOME_SCHEMA_V2 else 2
    if (
        record["schema"] != expected_schema
        or record["protocol_schema"] != PROTOCOL_MANIFEST_SCHEMA
        or record["manifest_id"] != manifest.manifest_id
        or record["engine_sha256"] != manifest.engine_sha256
        or record["operation"] != descriptor.operation
        or record["features_tuple"] != features
        or record["request_schema"] != descriptor.request_schema
        or record["outcome_schema"] != descriptor.outcome_schema
        or record["receipt_schema"] != descriptor.receipt_schema
        or record["verifier_schema"] != descriptor.verifier_schema
        or record["observation_subject"] != descriptor.observation_subject
        or record["revision_algorithm"] != descriptor.revision_algorithm
        or record["revision_policy"] != descriptor.revision_policy
        or record["requested_target_revision"] != requested_revision
        or record["revision_binding"] != "bound"
        or record["execution_binding"] != descriptor.execution_binding
        or record["observation_to_exec_channel"]
        != descriptor.observation_to_exec_channel
        or record["tier"] != "TIER2"
        or record["grade"] != "D2"
        or record["scope"] != SCOPE
        or record["oracle"] != "cooperative-llm-call-completed"
        or not record["authentic"]
        or record["exit_code"] != 0
        or record["executions"] != expected_executions
    ):
        raise ValueError("v2 machine record does not match the negotiated request")
    for field in (
        "manifest_id",
        "engine_sha256",
        "claimed_observed_revision",
        "fresh_observed_revision",
        "verified_observed_revision",
        "engine_request_id",
        "evidence_id",
        "receipt_sha256",
        "verification_result_id",
    ):
        if not _is_lower_hex(record[field], 64):
            raise ValueError(f"v2 machine record has malformed {field}")
    if not _is_lower_hex(record["result_digest"], 32):
        raise ValueError("v2 machine record has malformed result digest")
    cassette_prefix = "vh-cassette-v2:sha256:"
    if not (
        record["cassette_identity"].startswith(cassette_prefix)
        and _is_lower_hex(record["cassette_identity"][len(cassette_prefix) :], 64)
    ):
        raise ValueError("v2 machine record has malformed cassette identity")
    if not (
        record["claimed_observed_revision"]
        == record["fresh_observed_revision"]
        == record["verified_observed_revision"]
    ):
        raise ValueError("v2 claimed/fresh/verified revision equality failed")
    requested_prefix = f"{descriptor.revision_algorithm}:"
    requested = record["requested_target_revision"]
    if not requested.startswith(requested_prefix):
        raise ValueError("bound-required v2 record lacks an exact requested revision")
    requested_digest = requested[len(requested_prefix) :]
    if not _is_lower_hex(requested_digest, 64):
        raise ValueError("bound-required v2 record has malformed requested revision")
    if requested_digest != record["verified_observed_revision"]:
        raise ValueError("requested revision does not equal the verified observation")
    if expected_schema == COOPERATIVE_OUTCOME_SCHEMA_V2:
        if process_returncode != record["outcome_exit_code"]:
            raise ValueError("v2 initial status does not match verified outcome")
    elif process_returncode != record["exit_code"]:
        raise ValueError("v2 verifier process status mismatch")
    shapes = {
        "CLEAN": (0, 0, "completed", "none", True),
        "FINDINGS": (
            1,
            1,
            "not-completed:timeout",
            "cooperative-llm-call-completed:timeout",
            True,
        ),
        "UNCHECKED": (3, 0, "indeterminate", "none", False),
    }
    shape = shapes.get(record["verdict"])
    if shape is None or shape != (
        record["outcome_exit_code"],
        record["findings_count"],
        record["oracle_evaluation"],
        record["finding_identity"],
        record["verified"],
    ):
        raise ValueError("v2 machine record has an impossible verdict shape")
    if record["verdict"] in {"CLEAN", "FINDINGS"} and record["errors"]:
        raise ValueError("checked v2 outcome carries errors")


def _protocol_descriptor_for(
    manifest: _ProtocolManifest, requirement: ProtocolRequirement
) -> Optional[_ProtocolDescriptor]:
    return next(
        (
            descriptor
            for descriptor in manifest.descriptors
            if descriptor.operation == requirement.operation.value
        ),
        None,
    )


def _negotiated_feature_closure(
    requirement: ProtocolRequirement,
    descriptor: Optional[_ProtocolDescriptor],
) -> Tuple[str, ...]:
    extras = tuple(feature.value for feature in requirement.required_features)
    if descriptor is None:
        return extras
    return tuple(sorted(set(descriptor.mandatory_features).union(extras)))


def _validate_revision_coordinate(
    requirement: ProtocolRequirement, descriptor: _ProtocolDescriptor
) -> None:
    revision = requirement.requested_target_revision
    if (
        revision.subject != descriptor.observation_subject
        or revision.algorithm != descriptor.revision_algorithm
    ):
        raise ValueError(
            "requested target revision coordinate differs from the engine descriptor"
        )


def _decode_process_bytes(value: bytes) -> str:
    return value.decode("utf-8", errors="replace")


def _v2_protocol_report(record: Dict[str, Any]) -> ProtocolReport:
    return ProtocolReport(
        manifest_id=record["manifest_id"],
        operation=record["operation"],
        features=record["features_tuple"],
        engine_request_id=record["engine_request_id"],
        receipt_schema=record["receipt_schema"],
        verify_schema=record["verifier_schema"],
    )


def _v2_revision_report(record: Dict[str, Any]) -> RevisionReportRecord:
    return RevisionReport(
        observation_subject=record["observation_subject"],
        revision_algorithm=record["revision_algorithm"],
        revision_policy=record["revision_policy"],
        requested=record["requested_target_revision"],
        claimed_observed=record["claimed_observed_revision"],
        fresh_observed=record["fresh_observed_revision"],
        verified_observed=record["verified_observed_revision"],
        binding=record["revision_binding"],
        execution_binding=record["execution_binding"],
        observation_to_exec_channel=record["observation_to_exec_channel"],
    )


class _DuplicateJsonKey(ValueError):
    pass


def _strict_json_object(line: str) -> Optional[Dict[str, Any]]:
    def reject_duplicates(pairs):
        result: Dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise _DuplicateJsonKey(key)
            result[key] = value
        return result

    value = json.loads(line, object_pairs_hook=reject_duplicates)
    return value if type(value) is dict else None


def _parse_single_machine_record(
    stdout: str, *, record: str, schema: str
) -> Optional[Dict[str, Any]]:
    line = stdout[:-1] if stdout.endswith("\n") else stdout
    if not line or "\n" in line or "\r" in line or line != line.strip():
        return None
    try:
        rec = _strict_json_object(line)
    except (ValueError, RecursionError):
        return None
    if rec is None or rec.get("record") != record or rec.get("schema") != schema:
        return None
    return rec


def _parse_verify_run(stdout: str) -> Optional[Dict[str, Any]]:
    return _parse_single_machine_record(
        stdout, record="verify-run", schema=VERIFY_RUN_SCHEMA
    )


def _parse_cooperative_verify(stdout: str) -> Optional[Dict[str, Any]]:
    return _parse_single_machine_record(
        stdout, record="cooperative-verify", schema=VERIFY_COOPERATIVE_SCHEMA
    )


def _build_cooperative_args(request: RunRequest, out_dir: Path) -> List[str]:
    args: List[str] = [
        "cooperative",
        "--workload",
        request.workload,
        "--out",
        str(out_dir),
    ]
    if request.cassette_path is not None:
        args.extend(["--cassette", request.cassette_path])
    return args


def _requested_revision_wire(requirement: ProtocolRequirement) -> str:
    return requirement.requested_target_revision.coordinate


def _build_cooperative_v2_args(
    request: RunRequest,
    out_dir: Path,
    manifest: _ProtocolManifest,
    features: Tuple[str, ...],
) -> List[str]:
    requirement = request.protocol_requirement
    if requirement is None:
        raise ValueError("negotiated cooperative request is missing requirements")
    args = [
        "cooperative-v2",
        "--protocol-schema",
        PROTOCOL_MANIFEST_SCHEMA,
        "--manifest-id",
        manifest.manifest_id,
        "--operation",
        requirement.operation.value,
    ]
    for feature in features:
        args.extend(["--require-feature", feature])
    args.extend(
        [
            "--requested-target-revision",
            _requested_revision_wire(requirement),
            "--out",
            str(out_dir),
        ]
    )
    if request.cassette_path is not None:
        args.extend(["--cassette", request.cassette_path])
    return args


def _build_verify_cooperative_v2_args(
    request: RunRequest,
    receipt: Path,
    manifest: _ProtocolManifest,
    descriptor: _ProtocolDescriptor,
    features: Tuple[str, ...],
) -> List[str]:
    requirement = request.protocol_requirement
    if requirement is None:
        raise ValueError("v2 reverification requires negotiated request data")
    args = [
        "verify-cooperative-v2",
        "--receipt",
        str(receipt),
        "--expected-operation",
        requirement.operation.value,
    ]
    for feature in features:
        args.extend(["--expected-feature", feature])
    args.extend(
        [
            "--expected-requested-target-revision",
            _requested_revision_wire(requirement),
            "--expected-protocol-schema",
            PROTOCOL_MANIFEST_SCHEMA,
            "--expected-manifest-id",
            manifest.manifest_id,
            "--expected-request-schema",
            descriptor.request_schema,
            "--expected-outcome-schema",
            descriptor.outcome_schema,
            "--expected-receipt-schema",
            descriptor.receipt_schema,
            "--expected-verifier-schema",
            descriptor.verifier_schema,
            "--expected-observation-subject",
            descriptor.observation_subject,
            "--expected-revision-algorithm",
            descriptor.revision_algorithm,
            "--expected-revision-policy",
            descriptor.revision_policy,
            "--expected-execution-binding",
            descriptor.execution_binding,
            "--expected-observation-to-exec-channel",
            descriptor.observation_to_exec_channel,
        ]
    )
    if request.cassette_path is None:
        args.append("--expect-default-cassette")
    else:
        args.extend(["--expected-cassette", request.cassette_path])
    return args


def _is_lower_hex(value: Any, length: int) -> bool:
    return (
        type(value) is str
        and len(value) == length
        and all(char in "0123456789abcdef" for char in value)
    )


def _strict_errors(errors_field: Any) -> Optional[List[str]]:
    if type(errors_field) is not str:
        return None
    try:
        decoded = json.loads(errors_field)
    except (ValueError, RecursionError):
        return None
    if (
        type(decoded) is not list
        or len(decoded) > MAX_DIAGNOSTIC_ITEMS
        or any(type(item) is not str for item in decoded)
    ):
        return None
    try:
        if _canonical_json_bytes(decoded).decode("utf-8") != errors_field:
            return None
        if any(
            len(item.encode("utf-8", errors="strict")) > MAX_DIAGNOSTIC_BYTES
            or any(
                not char.isascii() or unicodedata.category(char) == "Cc"
                for char in item
            )
            for item in decoded
        ):
            return None
    except UnicodeEncodeError:
        return None
    return decoded


def _is_strict_utf8(value: Any) -> bool:
    if type(value) is not str:
        return False
    try:
        value.encode("utf-8", errors="strict")
    except UnicodeEncodeError:
        return False
    return True


_VERIFY_RUN_FIELDS = {
    "record",
    "schema",
    "authentic",
    "verified",
    "outcome_verified",
    "verdict",
    "outcome_exit_code",
    "evidence_digest",
    "result_digest",
    "engine_sha256",
    "engine_request_digest",
    "findings_total",
    "findings_verified",
    "errors",
}


def _validate_verify_run_record(
    rec: Dict[str, Any], process_returncode: int
) -> Optional[List[str]]:
    if set(rec) != _VERIFY_RUN_FIELDS:
        return None
    if rec["record"] != "verify-run" or rec["schema"] != VERIFY_RUN_SCHEMA:
        return None
    for field in ("authentic", "verified", "outcome_verified"):
        if type(rec[field]) is not bool:
            return None
    if not _is_strict_utf8(rec["verdict"]) or rec["verdict"] not in {
        "CLEAN",
        "FINDINGS",
        "UNCHECKED",
        "ERROR",
    }:
        return None
    if type(rec["outcome_exit_code"]) is not int or rec["outcome_exit_code"] not in {
        0,
        1,
        2,
        3,
    }:
        return None
    if not _is_lower_hex(rec["result_digest"], 64) or not _is_lower_hex(
        rec["engine_sha256"], 64
    ):
        return None
    for field in ("evidence_digest", "engine_request_digest"):
        if rec["authentic"]:
            if not _is_lower_hex(rec[field], 64):
                return None
        elif rec[field] != "" and not _is_lower_hex(rec[field], 64):
            return None
    for field in ("findings_total", "findings_verified"):
        if type(rec[field]) is not int or rec[field] < 0:
            return None
    if rec["findings_verified"] > rec["findings_total"]:
        return None
    errors = _strict_errors(rec["errors"])
    if errors is None:
        return None
    if process_returncode not in {0, 1} or rec["authentic"] != (process_returncode == 0):
        return None
    if rec["verified"] != (rec["authentic"] and rec["outcome_verified"]):
        return None
    if rec["authentic"]:
        semantic_shapes = {
            "CLEAN": (0, True),
            "FINDINGS": (1, True),
            "UNCHECKED": (3, False),
        }
        if semantic_shapes.get(rec["verdict"]) != (
            rec["outcome_exit_code"],
            rec["outcome_verified"],
        ):
            return None
        if errors or rec["findings_verified"] != rec["findings_total"]:
            return None
    elif (
        rec["verdict"] != "ERROR"
        or rec["outcome_exit_code"] != 2
        or rec["outcome_verified"]
        or rec["verified"]
        or not errors
    ):
        return None
    return errors


_COOPERATIVE_VERIFY_FIELDS = {
    "record",
    "schema",
    "verdict",
    "tier",
    "grade",
    "scope",
    "workload",
    "cassette_identity",
    "child_source_digest",
    "engine_request_digest",
    "oracle",
    "oracle_evaluation",
    "finding_identity",
    "findings_count",
    "evidence_digest",
    "result_digest",
    "engine_sha256",
    "receipt_sha256",
    "authentic",
    "verified",
    "outcome_exit_code",
    "exit_code",
    "errors",
}


def _validate_cooperative_verify_record(
    rec: Dict[str, Any], process_returncode: int, expected_workload: Optional[str]
) -> Optional[List[str]]:
    if set(rec) != _COOPERATIVE_VERIFY_FIELDS:
        return None
    if (
        rec["record"] != "cooperative-verify"
        or rec["schema"] != VERIFY_COOPERATIVE_SCHEMA
        or rec["tier"] != "TIER2"
        or rec["grade"] != "D2"
        or rec["scope"] != SCOPE
        or rec["oracle"] != "cooperative-llm-call-completed"
    ):
        return None
    if not _is_strict_utf8(rec["workload"]) or (
        expected_workload is not None and rec["workload"] != expected_workload
    ):
        return None
    cassette_identity = rec["cassette_identity"]
    if not (
        type(cassette_identity) is str
        and cassette_identity.startswith("vh-cassette-v2:sha256:")
        and _is_lower_hex(cassette_identity.removeprefix("vh-cassette-v2:sha256:"), 64)
    ):
        return None
    source_digest = rec["child_source_digest"]
    if not (
        type(source_digest) is str
        and source_digest.startswith("sha256:")
        and _is_lower_hex(source_digest.removeprefix("sha256:"), 64)
    ):
        return None
    for field in ("engine_request_digest", "engine_sha256", "receipt_sha256"):
        if not _is_lower_hex(rec[field], 64):
            return None
    for field in ("evidence_digest", "result_digest"):
        if not _is_lower_hex(rec[field], 32):
            return None
    for field in ("authentic", "verified"):
        if type(rec[field]) is not bool:
            return None
    for field in ("findings_count", "outcome_exit_code", "exit_code"):
        if type(rec[field]) is not int or rec[field] < 0:
            return None
    errors = _strict_errors(rec["errors"])
    if errors is None:
        return None

    if process_returncode not in {0, 1} or rec["exit_code"] != process_returncode:
        return None
    if rec["authentic"] != (process_returncode == 0):
        return None
    if not rec["authentic"] and not errors:
        return None

    if type(rec["verdict"]) is not str:
        return None
    semantic_shapes = {
        "CLEAN": (0, 0, "completed", "none", True),
        "FINDINGS": (
            1,
            1,
            "not-completed:timeout",
            "cooperative-llm-call-completed:timeout",
            True,
        ),
        "UNCHECKED": (3, 0, "indeterminate", "none", False),
    }
    shape = semantic_shapes.get(rec["verdict"])
    if shape is None:
        return None
    outcome_exit, findings, evaluation, finding, outcome_verified = shape
    if (
        rec["outcome_exit_code"] != outcome_exit
        or rec["findings_count"] != findings
        or rec["oracle_evaluation"] != evaluation
        or rec["finding_identity"] != finding
        or rec["verified"] != (rec["authentic"] and outcome_verified)
    ):
        return None
    if rec["verdict"] in {"CLEAN", "FINDINGS"} and errors:
        return None
    return errors


def _error_outcome(
    errors: List[str], *, receipt_dir: Optional[Path] = None
) -> OutcomeRecord:
    return Outcome(
        verdict=Verdict.ERROR,
        tier=Tier.UNKNOWN,
        grade=Grade.UNTRUSTED,
        scope=SCOPE,
        errors=list(errors),
        receipt_dir=str(receipt_dir) if receipt_dir is not None else None,
    )


def _v2_error_outcome(
    errors: List[str],
    *,
    untrusted: bool,
    receipt_dir: Optional[Path] = None,
    request_digest: Optional[str] = None,
    stdout: str = "",
    stderr: str = "",
    exit_code: int = -1,
    raw: Optional[Dict[str, Any]] = None,
) -> OutcomeRecord:
    return Outcome(
        verdict=Verdict.ERROR,
        tier=Tier.TIER2,
        grade=Grade.UNTRUSTED if untrusted else Grade.D2,
        scope=SCOPE,
        request_digest=request_digest,
        receipt_dir=str(receipt_dir) if receipt_dir is not None else None,
        stdout=stdout,
        stderr=stderr,
        exit_code=exit_code,
        verified=False,
        errors=list(errors),
        raw={} if raw is None else raw,
    )


class MultiverseRunner:
    """Client-only adapter to the Rust vibe-halt engine.

    Python is never a second simulator: it constructs an untrusted caller
    envelope and maps only closed, validated Rust machine records. The
    resulting Python object is process-local data, not an authority boundary.
    """

    def __init__(self, policy: EnginePolicy):
        if type(policy) is not EnginePolicy:
            raise TypeError("policy must be an EnginePolicy instance")
        # Keep a validated base-class snapshot. A caller retaining `policy`
        # cannot mutate it with `object.__setattr__` after runner creation and
        # thereby change the trust root mid-invocation.
        self._policy = EnginePolicy(**asdict(policy))

    def run(self, request: RunRequest) -> OutcomeRecord:
        if type(self) is not MultiverseRunner:
            raise TypeError("runner subclasses are not an authority boundary")
        if type(request) is not RunRequest:
            raise TypeError("request must be a RunRequest instance")

        # Frozen dataclasses can still be mutated through low-level Python
        # mechanisms. Reconstruct one exact base-class instance at the entry
        # boundary, which reruns strict type/contract validation, and use only
        # that immutable snapshot for hashing and argv construction.
        try:
            request = _snapshot_run_request(request)
        except (AttributeError, TypeError, ValueError) as exc:
            return _error_outcome([f"invalid request at run boundary: {exc}"])

        try:
            out_dir = _prepare_output_root(request)
        except (OSError, ValueError) as e:
            return _error_outcome([str(e)])

        try:
            engine_lease, engine_dir = _private_engine_directory("vibe-halt-engine-")
        except (OSError, ValueError):
            return _error_outcome(
                ["private engine directory refused"], receipt_dir=out_dir
            )
        with engine_lease:
            return self._run_with_private_engine(request, out_dir, engine_dir)

    def _run_with_private_engine(
        self, request: RunRequest, out_dir: Path, engine_dir: Path
    ) -> OutcomeRecord:
        try:
            engine, untrusted, copied_engine_digest = _copy_and_verify_engine(
                self._policy, engine_dir
            )
        except ValueError as e:
            return _error_outcome([str(e)], receipt_dir=out_dir)

        invocation_id = request.invocation_id or secrets.token_hex(16)
        request_dict = _request_dict(request)
        request_digest = _sha256_hex(_canonical_json_bytes(request_dict))

        manifest: Optional[_ProtocolManifest] = None
        descriptor: Optional[_ProtocolDescriptor] = None
        features: Tuple[str, ...] = ()
        if request.protocol_requirement is not None:
            try:
                manifest = _query_protocol_manifest(
                    engine, engine_dir, copied_engine_digest
                )
            except ValueError as exc:
                return _error_outcome(
                    [f"protocol manifest refused: {exc}"], receipt_dir=out_dir
                )
            descriptor = _protocol_descriptor_for(
                manifest, request.protocol_requirement
            )
            # An unsupported operation still goes to Rust so only the engine
            # can issue a typed negotiation refusal. For a known descriptor,
            # the caller extras are unioned with the mandatory closure; an
            # empty caller tuple can never weaken the operation.
            features = _negotiated_feature_closure(
                request.protocol_requirement, descriptor
            )
            request_dict["negotiated_manifest_id"] = manifest.manifest_id
            request_dict["negotiated_features"] = list(features)

        if request.transport == "cooperative":
            if request.protocol_requirement is not None:
                if manifest is None:
                    return _error_outcome(
                        ["negotiated cooperative request has no engine manifest"],
                        receipt_dir=out_dir,
                    )
                return self._run_cooperative_v2(
                    request,
                    out_dir,
                    engine,
                    engine_dir,
                    copied_engine_digest,
                    untrusted,
                    invocation_id,
                    request_dict,
                    request_digest,
                    manifest,
                    descriptor,
                    features,
                )
            return self._run_cooperative(
                request,
                out_dir,
                engine,
                engine_dir,
                untrusted,
                invocation_id,
                request_dict,
                request_digest,
            )

        run_proc = _invoke_engine(
            [str(engine)] + _build_run_args(request, out_dir), cwd=engine_dir
        )
        if run_proc.returncode not in {0, 1, 3}:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED if untrusted else Grade.D0,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(out_dir),
                stdout=run_proc.stdout,
                stderr=run_proc.stderr,
                exit_code=run_proc.returncode,
                verified=False,
                errors=["engine run failed before producing an admissible outcome"],
            )

        verify_proc = _invoke_engine(
            [
                str(engine),
                "verify-run",
                "--out",
                str(out_dir),
                "--engine",
                str(engine),
            ],
            cwd=engine_dir,
        )
        verify_rec = _parse_verify_run(verify_proc.stdout)
        verify_errors = (
            None
            if verify_rec is None
            else _validate_verify_run_record(verify_rec, verify_proc.returncode)
        )
        if verify_rec is None or verify_errors is None:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED if untrusted else Grade.D0,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(out_dir),
                stdout=run_proc.stdout + "\n" + verify_proc.stdout,
                stderr=run_proc.stderr + "\n" + verify_proc.stderr,
                exit_code=verify_proc.returncode,
                verified=False,
                errors=["verify-run produced no valid machine record"],
            )

        evidence_digest = verify_rec.get("evidence_digest")
        result_digest = verify_rec.get("result_digest")
        engine_verdict = verify_rec.get("verdict", "UNCHECKED")
        findings_total = verify_rec["findings_total"]
        verified = verify_rec["verified"]

        raw: Dict[str, Any] = dict(verify_rec)
        raw["run_stdout"] = run_proc.stdout
        raw["run_stderr"] = run_proc.stderr
        raw["run_exit_code"] = run_proc.returncode

        if not verify_rec["authentic"]:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED if untrusted else Grade.D0,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(out_dir),
                evidence_digest=evidence_digest,
                stdout=run_proc.stdout,
                stderr=run_proc.stderr,
                exit_code=verify_rec["outcome_exit_code"],
                verified=False,
                errors=verify_errors,
                findings_count=findings_total,
                raw=raw,
            )

        if (
            not untrusted
            and verify_rec["engine_sha256"] != self._policy.expected_digest
        ):
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.D0,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(out_dir),
                evidence_digest=evidence_digest,
                stdout=run_proc.stdout,
                stderr=run_proc.stderr,
                exit_code=1,
                verified=False,
                errors=["verifier engine digest does not match the configured trust root"],
                findings_count=findings_total,
                raw=raw,
            )

        expected_engine_request = _generic_engine_request_digest(request)
        if verify_rec["engine_request_digest"] != expected_engine_request:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED if untrusted else Grade.D0,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(out_dir),
                evidence_digest=evidence_digest,
                stdout=run_proc.stdout,
                stderr=run_proc.stderr,
                exit_code=1,
                verified=False,
                errors=["verified receipt request does not match the invocation request"],
                findings_count=findings_total,
                raw=raw,
            )
        if run_proc.returncode != verify_rec["outcome_exit_code"]:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED if untrusted else Grade.D0,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(out_dir),
                evidence_digest=evidence_digest,
                stdout=run_proc.stdout,
                stderr=run_proc.stderr,
                exit_code=1,
                verified=False,
                errors=["initial engine status does not match fresh reverification"],
                findings_count=findings_total,
                raw=raw,
            )

        envelope = {
            "invocation_id": invocation_id,
            "request": request_dict,
            "engine_policy": {
                "path": str(engine),
                "expected_digest": self._policy.expected_digest,
            },
            "output_root": str(out_dir),
            "result_digest": result_digest,
            "evidence_digest": evidence_digest,
            "engine_sha256": verify_rec["engine_sha256"],
            "engine_request_digest": verify_rec["engine_request_digest"],
        }
        invocation_envelope_digest = _sha256_hex(_canonical_json_bytes(envelope))

        if untrusted:
            return Outcome(
                verdict=Verdict.UNCHECKED,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED,
                scope=SCOPE,
                request_digest=request_digest,
                invocation_envelope_digest=invocation_envelope_digest,
                evidence_digest=evidence_digest,
                receipt_dir=str(out_dir),
                stdout=run_proc.stdout,
                stderr=run_proc.stderr,
                exit_code=run_proc.returncode,
                verified=False,
                errors=["no engine trust root configured; checked verdict refused"],
                findings_count=findings_total,
                raw=raw,
            )

        if not verified:
            return Outcome(
                verdict=Verdict.UNCHECKED,
                tier=Tier.TIER1,
                grade=Grade.D0,
                scope=SCOPE,
                request_digest=request_digest,
                invocation_envelope_digest=invocation_envelope_digest,
                evidence_digest=evidence_digest,
                receipt_dir=str(out_dir),
                stdout=run_proc.stdout,
                stderr=run_proc.stderr,
                exit_code=verify_rec["outcome_exit_code"],
                verified=False,
                errors=["fresh reproduction remained semantically unchecked"],
                findings_count=findings_total,
                raw=raw,
            )

        verdict = Verdict(engine_verdict)
        return Outcome(
            verdict=verdict,
            tier=Tier.TIER1,
            grade=Grade.D0,
            scope=SCOPE,
            request_digest=request_digest,
            invocation_envelope_digest=invocation_envelope_digest,
            evidence_digest=evidence_digest,
            receipt_dir=str(out_dir),
            stdout=run_proc.stdout,
            stderr=run_proc.stderr,
            exit_code=verify_rec["outcome_exit_code"],
            verified=verified,
            findings_count=findings_total,
            raw=raw,
        )

    def _map_v2_refusal(
        self,
        process: subprocess.CompletedProcess[bytes],
        *,
        manifest: _ProtocolManifest,
        copied_engine_digest: str,
        untrusted: bool,
        receipt_dir: Path,
        request_digest: Optional[str],
    ) -> OutcomeRecord:
        stdout = _decode_process_bytes(process.stdout)
        stderr = _decode_process_bytes(process.stderr)
        try:
            if process.returncode != 4 or process.stderr:
                raise ValueError("refusal did not use the canonical status and stderr shape")
            refusal = _parse_engine_refusal(process.stdout)
            if (
                refusal.engine_sha256 != copied_engine_digest
                or refusal.engine_sha256 != manifest.engine_sha256
                or refusal.manifest_id != manifest.manifest_id
                or refusal.executions != 0
            ):
                raise ValueError("refusal is not bound to the queried copied engine")
        except ValueError as exc:
            return _v2_error_outcome(
                [f"negotiation refusal record invalid: {exc}"],
                untrusted=untrusted,
                receipt_dir=receipt_dir,
                request_digest=request_digest,
                stdout=stdout,
                stderr=stderr,
                exit_code=process.returncode,
            )

        report = RefusalReport(
            reason=refusal.reason,
            executions=refusal.executions,
            manifest_id=refusal.manifest_id,
        )
        raw = {
            "schema": ENGINE_REFUSAL_SCHEMA,
            "refusal": refusal.reason,
            "reason": refusal.reason,
            "engine_sha256": refusal.engine_sha256,
            "manifest_id": refusal.manifest_id,
            "executions": refusal.executions,
        }
        return Outcome(
            verdict=Verdict.ERROR,
            tier=Tier.TIER2,
            grade=Grade.UNTRUSTED if untrusted else Grade.D2,
            scope=SCOPE,
            request_digest=request_digest,
            receipt_dir=str(receipt_dir),
            stdout=stdout,
            stderr=stderr,
            exit_code=4,
            verified=False,
            errors=[f"engine negotiation refused: {refusal.reason}"],
            refusal=report,
            raw=raw,
        )

    def _run_cooperative_v2(
        self,
        request: RunRequest,
        out_dir: Path,
        engine: Path,
        engine_dir: Path,
        copied_engine_digest: str,
        untrusted: bool,
        invocation_id: str,
        request_dict: Dict[str, Any],
        request_digest: str,
        manifest: _ProtocolManifest,
        descriptor: Optional[_ProtocolDescriptor],
        features: Tuple[str, ...],
    ) -> OutcomeRecord:
        """Run negotiated cooperative v2 and map only strict Rust records."""

        requirement = request.protocol_requirement
        if requirement is None:
            return _v2_error_outcome(
                ["negotiated cooperative request is incomplete"],
                untrusted=untrusted,
                receipt_dir=out_dir,
                request_digest=request_digest,
            )
        if descriptor is not None:
            try:
                _validate_revision_coordinate(requirement, descriptor)
            except ValueError as exc:
                return _v2_error_outcome(
                    [str(exc)],
                    untrusted=untrusted,
                    receipt_dir=out_dir,
                    request_digest=request_digest,
                )

        process = _invoke_engine_bytes(
            [str(engine)]
            + _build_cooperative_v2_args(request, out_dir, manifest, features),
            cwd=engine_dir,
        )
        if process.returncode == 4:
            return self._map_v2_refusal(
                process,
                manifest=manifest,
                copied_engine_digest=copied_engine_digest,
                untrusted=untrusted,
                receipt_dir=out_dir,
                request_digest=request_digest,
            )

        stdout = _decode_process_bytes(process.stdout)
        stderr = _decode_process_bytes(process.stderr)
        if process.returncode not in {0, 1, 3} or process.stderr:
            return _v2_error_outcome(
                ["cooperative-v2 produced no admissible machine outcome"],
                untrusted=untrusted,
                receipt_dir=out_dir,
                request_digest=request_digest,
                stdout=stdout,
                stderr=stderr,
                exit_code=process.returncode,
            )
        if descriptor is None:
            return _v2_error_outcome(
                ["engine accepted an operation absent from its queried manifest"],
                untrusted=untrusted,
                receipt_dir=out_dir,
                request_digest=request_digest,
                stdout=stdout,
                stderr=stderr,
                exit_code=process.returncode,
            )
        try:
            initial_record = _parse_v2_machine_record(
                process.stdout, COOPERATIVE_OUTCOME_SCHEMA_V2
            )
            _validate_v2_machine_record(
                initial_record,
                expected_schema=COOPERATIVE_OUTCOME_SCHEMA_V2,
                process_returncode=process.returncode,
                manifest=manifest,
                descriptor=descriptor,
                features=features,
                requested_revision=_requested_revision_wire(requirement),
            )
        except ValueError as exc:
            return _v2_error_outcome(
                [f"cooperative-v2 machine outcome invalid: {exc}"],
                untrusted=untrusted,
                receipt_dir=out_dir,
                request_digest=request_digest,
                stdout=stdout,
                stderr=stderr,
                exit_code=process.returncode,
            )

        receipt = out_dir / COOPERATIVE_RECEIPT_NAME
        if not _regular_file_without_links(receipt):
            return _v2_error_outcome(
                ["cooperative-v2 did not persist a regular no-link receipt"],
                untrusted=untrusted,
                receipt_dir=out_dir,
                request_digest=request_digest,
                stdout=stdout,
                stderr=stderr,
                exit_code=process.returncode,
            )
        return self._verify_cooperative_v2_receipt(
            receipt,
            engine,
            engine_dir,
            request=request,
            manifest=manifest,
            descriptor=descriptor,
            features=features,
            copied_engine_digest=copied_engine_digest,
            untrusted=untrusted,
            receipt_dir=out_dir,
            request_digest=request_digest,
            invocation_id=invocation_id,
            request_dict=request_dict,
            initial_record=initial_record,
            initial_exit_code=process.returncode,
            prior_stdout=process.stdout,
            prior_stderr=process.stderr,
        )

    def _verify_cooperative_v2_receipt(
        self,
        receipt: Path,
        engine: Path,
        cwd: Path,
        *,
        request: RunRequest,
        manifest: _ProtocolManifest,
        descriptor: _ProtocolDescriptor,
        features: Tuple[str, ...],
        copied_engine_digest: str,
        untrusted: bool,
        receipt_dir: Path,
        request_digest: Optional[str] = None,
        invocation_id: Optional[str] = None,
        request_dict: Optional[Dict[str, Any]] = None,
        initial_record: Optional[Dict[str, Any]] = None,
        initial_exit_code: Optional[int] = None,
        prior_stdout: bytes = b"",
        prior_stderr: bytes = b"",
    ) -> OutcomeRecord:
        requirement = request.protocol_requirement
        if requirement is None:
            return _v2_error_outcome(
                ["v2 reverification requires independent negotiated request data"],
                untrusted=untrusted,
                receipt_dir=receipt_dir,
                request_digest=request_digest,
            )
        process = _invoke_engine_bytes(
            [str(engine)]
            + _build_verify_cooperative_v2_args(
                request, receipt, manifest, descriptor, features
            ),
            cwd=cwd,
        )
        if process.returncode == 4:
            return self._map_v2_refusal(
                process,
                manifest=manifest,
                copied_engine_digest=copied_engine_digest,
                untrusted=untrusted,
                receipt_dir=receipt_dir,
                request_digest=request_digest,
            )

        stdout = _decode_process_bytes(prior_stdout + process.stdout)
        stderr = _decode_process_bytes(prior_stderr + process.stderr)
        if process.returncode == 1 and not process.stderr:
            try:
                failure = _parse_v2_verification_failure(process.stdout)
                if (
                    failure.engine_sha256 != copied_engine_digest
                    or failure.engine_sha256 != manifest.engine_sha256
                    or failure.manifest_id != manifest.manifest_id
                ):
                    raise ValueError(
                        "verification failure does not bind the copied engine manifest"
                    )
                if failure.reason == "fresh-replay-failed":
                    if failure.executions == 0:
                        raise ValueError(
                            "fresh replay failure reports no admitted sandbox attempt"
                        )
                elif failure.executions != 0:
                    raise ValueError(
                        "pre-replay verification failure crossed the sandbox boundary"
                    )
                if failure.receipt_sha256 == "unavailable":
                    try:
                        _read_bounded_regular_file(
                            receipt, MAX_COOPERATIVE_RECEIPT_BYTES
                        )
                    except ValueError:
                        pass
                    else:
                        raise ValueError(
                            "verification failure omitted an available receipt identity"
                        )
                else:
                    receipt_bytes = _read_bounded_regular_file(
                        receipt, MAX_COOPERATIVE_RECEIPT_BYTES
                    )
                    if _sha256_hex(receipt_bytes) != failure.receipt_sha256:
                        raise ValueError(
                            "verification failure receipt identity mismatch"
                        )
            except ValueError as exc:
                return _v2_error_outcome(
                    [f"verify-cooperative-v2 failure record invalid: {exc}"],
                    untrusted=untrusted,
                    receipt_dir=receipt_dir,
                    request_digest=request_digest,
                    stdout=stdout,
                    stderr=stderr,
                    exit_code=process.returncode,
                )
            raw = {
                "schema": COOPERATIVE_VERIFY_FAILURE_SCHEMA_V1,
                "verification_failure": failure.reason,
                "reason": failure.reason,
                "engine_sha256": failure.engine_sha256,
                "manifest_id": failure.manifest_id,
                "receipt_sha256": failure.receipt_sha256,
                "executions": failure.executions,
                "authentic": failure.authentic,
                "verified": failure.verified,
                "exit_code": failure.exit_code,
            }
            return _v2_error_outcome(
                [f"verification failed: {failure.reason}"],
                untrusted=untrusted,
                receipt_dir=receipt_dir,
                request_digest=request_digest,
                stdout=stdout,
                stderr=stderr,
                exit_code=process.returncode,
                raw=raw,
            )
        if process.returncode != 0 or process.stderr:
            return _v2_error_outcome(
                ["verify-cooperative-v2 produced no valid machine record"],
                untrusted=untrusted,
                receipt_dir=receipt_dir,
                request_digest=request_digest,
                stdout=stdout,
                stderr=stderr,
                exit_code=process.returncode,
            )
        try:
            record = _parse_v2_machine_record(
                process.stdout, COOPERATIVE_VERIFY_SCHEMA_V2
            )
            _validate_v2_machine_record(
                record,
                expected_schema=COOPERATIVE_VERIFY_SCHEMA_V2,
                process_returncode=process.returncode,
                manifest=manifest,
                descriptor=descriptor,
                features=features,
                requested_revision=_requested_revision_wire(requirement),
            )
            if initial_exit_code is not None and (
                initial_record is None
                or initial_exit_code != record["outcome_exit_code"]
                or {
                    key: value
                    for key, value in initial_record.items()
                    if key not in {"schema", "executions"}
                }
                != {
                    key: value
                    for key, value in record.items()
                    if key not in {"schema", "executions"}
                }
            ):
                raise ValueError(
                    "initial outcome differs from independent strict reverification"
                )
        except ValueError as exc:
            return _v2_error_outcome(
                [f"verify-cooperative-v2 machine record invalid: {exc}"],
                untrusted=untrusted,
                receipt_dir=receipt_dir,
                request_digest=request_digest,
                stdout=stdout,
                stderr=stderr,
                exit_code=process.returncode,
            )

        raw = dict(record)
        raw["coop_stdout"] = _decode_process_bytes(prior_stdout)
        raw["coop_stderr"] = _decode_process_bytes(prior_stderr)
        raw["verify_stderr"] = _decode_process_bytes(process.stderr)
        protocol_report = _v2_protocol_report(record)
        revision_report = _v2_revision_report(record)

        invocation_envelope_digest = None
        if invocation_id is not None and request_dict is not None:
            envelope = {
                "invocation_id": invocation_id,
                "request": request_dict,
                "engine_policy": {
                    "path": str(engine),
                    "expected_digest": self._policy.expected_digest,
                },
                "output_root": str(receipt_dir),
                "result_digest": record["result_digest"],
                "evidence_digest": record["evidence_id"],
                "receipt_sha256": record["receipt_sha256"],
                "manifest_id": record["manifest_id"],
                "engine_request_id": record["engine_request_id"],
                "requested_target_revision": record[
                    "requested_target_revision"
                ],
                "verified_observed_revision": record[
                    "verified_observed_revision"
                ],
            }
            invocation_envelope_digest = _sha256_hex(_canonical_json_bytes(envelope))

        common: Dict[str, Any] = {
            "tier": Tier.TIER2,
            "scope": SCOPE,
            "request_digest": request_digest,
            "invocation_envelope_digest": invocation_envelope_digest,
            "evidence_digest": record["evidence_id"],
            "receipt_dir": str(receipt_dir),
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": record["outcome_exit_code"],
            "findings_count": record["findings_count"],
            "protocol": protocol_report,
            "revision": revision_report,
            "raw": raw,
        }
        if not record["verified"]:
            return Outcome(
                verdict=Verdict.UNCHECKED,
                grade=Grade.UNTRUSTED if untrusted else Grade.D2,
                verified=False,
                errors=record["errors"],
                **common,
            )
        if untrusted:
            return Outcome(
                verdict=Verdict.UNCHECKED,
                grade=Grade.UNTRUSTED,
                verified=False,
                errors=["no engine trust root configured; checked verdict refused"],
                **common,
            )
        return Outcome(
            verdict=Verdict(record["verdict"]),
            grade=Grade.D2,
            verified=True,
            errors=record["errors"],
            **common,
        )

    def _run_cooperative(
        self,
        request: RunRequest,
        out_dir: Path,
        engine: Path,
        engine_dir: Path,
        untrusted: bool,
        invocation_id: str,
        request_dict: Dict[str, Any],
        request_digest: str,
    ) -> OutcomeRecord:
        """Run a cooperative D2 transport workload through the Rust engine.

        The initial run's stdout is never trusted: the only outcome this
        adapter maps is the strict Rust reverifier's typed record over
        the persisted, bounded receipt (fresh replay + engine binding).
        """
        coop_proc = _invoke_engine(
            [str(engine)] + _build_cooperative_args(request, out_dir), cwd=engine_dir
        )
        if coop_proc.returncode not in {0, 1, 3}:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER2,
                grade=Grade.UNTRUSTED if untrusted else Grade.D2,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(out_dir),
                stdout=coop_proc.stdout,
                stderr=coop_proc.stderr,
                exit_code=coop_proc.returncode,
                verified=False,
                errors=["cooperative engine run returned an inadmissible status"],
            )
        receipt = out_dir / COOPERATIVE_RECEIPT_NAME
        if not _regular_file_without_links(receipt):
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER2,
                grade=Grade.UNTRUSTED if untrusted else Grade.D2,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(out_dir),
                stdout=coop_proc.stdout,
                stderr=coop_proc.stderr,
                exit_code=coop_proc.returncode,
                verified=False,
                errors=["cooperative run did not persist a receipt"],
            )
        return self._verify_cooperative_receipt(
            receipt,
            engine,
            engine_dir,
            untrusted=untrusted,
            tier=Tier.TIER2,
            request_digest=request_digest,
            invocation_id=invocation_id,
            request_dict=request_dict,
            receipt_dir=out_dir,
            prior_stdout=coop_proc.stdout,
            prior_stderr=coop_proc.stderr,
            expected_request=request,
            initial_exit_code=coop_proc.returncode,
        )

    def _verify_cooperative_receipt(
        self,
        receipt: Path,
        engine: Path,
        cwd: Path,
        *,
        untrusted: bool,
        tier: Tier,
        request_digest: Optional[str] = None,
        invocation_id: Optional[str] = None,
        request_dict: Optional[Dict[str, Any]] = None,
        receipt_dir: Optional[Path] = None,
        prior_stdout: str = "",
        prior_stderr: str = "",
        expected_request: Optional[RunRequest] = None,
        initial_exit_code: Optional[int] = None,
    ) -> OutcomeRecord:
        """Map ONLY the strict Rust cooperative reverifier's typed output.

        Authenticity and outcome verification are distinct in Rust. Invalid
        verifier evidence is ERROR. An authentic, freshly reproduced Rust
        UNCHECKED outcome remains unverified UNCHECKED with its diagnostics;
        it is never promoted through the no-root path. A configured trust root
        must match before a D2 verdict (including UNCHECKED) can be exposed.
        """
        verify_args = [str(engine), "verify-cooperative", "--receipt", str(receipt)]
        if expected_request is not None:
            verify_args.extend(["--expected-workload", expected_request.workload])
            if expected_request.cassette_path is None:
                verify_args.append("--expect-default-cassette")
            else:
                verify_args.extend(
                    ["--expected-cassette", expected_request.cassette_path]
                )
        verify_proc = _invoke_engine(verify_args, cwd=cwd)
        verify_rec = _parse_cooperative_verify(verify_proc.stdout)
        errors = (
            None
            if verify_rec is None
            else _validate_cooperative_verify_record(
                verify_rec,
                verify_proc.returncode,
                expected_request.workload if expected_request is not None else None,
            )
        )
        if verify_rec is None or errors is None:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=tier,
                grade=Grade.UNTRUSTED if untrusted else Grade.D2,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(receipt_dir) if receipt_dir else None,
                stdout=prior_stdout + verify_proc.stdout,
                stderr=prior_stderr + verify_proc.stderr,
                exit_code=verify_proc.returncode,
                verified=False,
                errors=["verify-cooperative produced no valid machine record"],
            )

        raw: Dict[str, Any] = dict(verify_rec)
        raw["protocol_generation"] = "legacy-v1"
        raw["revision_binding"] = "legacy-unbound"
        raw["coop_stdout"] = prior_stdout
        raw["coop_stderr"] = prior_stderr
        raw["verify_stderr"] = verify_proc.stderr
        legacy_revision = RevisionReport(
            observation_subject=None,
            revision_algorithm=None,
            revision_policy=None,
            requested=None,
            claimed_observed=None,
            fresh_observed=None,
            verified_observed=None,
            binding="legacy-unbound",
            execution_binding="staged-d2",
            observation_to_exec_channel="open",
        )
        if (
            initial_exit_code is not None
            and verify_rec["outcome_exit_code"] != initial_exit_code
        ):
            return Outcome(
                verdict=Verdict.ERROR,
                tier=tier,
                grade=Grade.UNTRUSTED if untrusted else Grade.D2,
                scope=SCOPE,
                request_digest=request_digest,
                receipt_dir=str(receipt_dir) if receipt_dir else None,
                stdout=prior_stdout + verify_proc.stdout,
                stderr=prior_stderr + verify_proc.stderr,
                exit_code=1,
                verified=False,
                errors=["initial cooperative status does not match strict reverification"],
                revision=legacy_revision,
                raw=raw,
            )

        evidence_digest = verify_rec["evidence_digest"]
        result_digest = verify_rec["result_digest"]
        engine_verdict = verify_rec["verdict"]
        findings_total = verify_rec["findings_count"]
        verified = verify_rec["verified"]

        invocation_envelope_digest = None
        if invocation_id is not None and request_dict is not None:
            envelope = {
                "invocation_id": invocation_id,
                "request": request_dict,
                "engine_policy": {
                    "path": str(engine),
                    "expected_digest": self._policy.expected_digest,
                },
                "output_root": str(receipt_dir) if receipt_dir else None,
                "result_digest": result_digest,
                "evidence_digest": evidence_digest,
                "receipt_sha256": verify_rec["receipt_sha256"],
                "engine_request_digest": verify_rec["engine_request_digest"],
                "verified_workload": verify_rec["workload"],
                "verified_cassette_identity": verify_rec["cassette_identity"],
            }
            invocation_envelope_digest = _sha256_hex(_canonical_json_bytes(envelope))

        # 1. Structurally or semantically inauthentic evidence is ERROR.
        if not verify_rec["authentic"]:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=tier,
                grade=Grade.UNTRUSTED if untrusted else Grade.D2,
                scope=SCOPE,
                request_digest=request_digest,
                invocation_envelope_digest=invocation_envelope_digest,
                evidence_digest=evidence_digest,
                receipt_dir=str(receipt_dir) if receipt_dir else None,
                stdout=prior_stdout + verify_proc.stdout,
                stderr=prior_stderr + verify_proc.stderr,
                exit_code=verify_rec["outcome_exit_code"],
                verified=False,
                errors=errors,
                findings_count=findings_total,
                revision=legacy_revision,
                raw=raw,
            )

        # 2. The verifier-reported engine digest must equal the configured
        #    trust root before any D2 verdict, including UNCHECKED, is mapped.
        verifier_engine = verify_rec.get("engine_sha256")
        if not untrusted and verifier_engine != self._policy.expected_digest:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=tier,
                grade=Grade.D2,
                scope=SCOPE,
                request_digest=request_digest,
                invocation_envelope_digest=invocation_envelope_digest,
                evidence_digest=evidence_digest,
                receipt_dir=str(receipt_dir) if receipt_dir else None,
                stdout=prior_stdout + verify_proc.stdout,
                stderr=prior_stderr + verify_proc.stderr,
                exit_code=1,
                verified=False,
                errors=[
                    "verifier-reported engine digest does not match the "
                    "configured trust root"
                ],
                findings_count=findings_total,
                revision=legacy_revision,
                raw=raw,
            )

        # 3. Authentic Rust UNCHECKED is a reproduced indeterminate result,
        #    not an invalid verifier record. Preserve its verdict and taint
        #    errors without ever marking it verified. Without a trust root it
        #    remains UNTRUSTED; with the matching root it is scoped D2.
        if not verified:
            return Outcome(
                verdict=Verdict.UNCHECKED,
                tier=tier,
                grade=Grade.UNTRUSTED if untrusted else Grade.D2,
                scope=SCOPE,
                request_digest=request_digest,
                invocation_envelope_digest=invocation_envelope_digest,
                evidence_digest=evidence_digest,
                receipt_dir=str(receipt_dir) if receipt_dir else None,
                stdout=prior_stdout + verify_proc.stdout,
                stderr=prior_stderr + verify_proc.stderr,
                exit_code=verify_rec["outcome_exit_code"],
                verified=False,
                errors=errors,
                findings_count=findings_total,
                revision=legacy_revision,
                raw=raw,
            )

        # 4. Without a trust root, even a successful internal replay carries
        #    no authority. It remains UNCHECKED and publicly unverified; the
        #    raw Rust record retains the narrower replay-consistency fact.
        if untrusted:
            return Outcome(
                verdict=Verdict.UNCHECKED,
                tier=tier,
                grade=Grade.UNTRUSTED,
                scope=SCOPE,
                request_digest=request_digest,
                invocation_envelope_digest=invocation_envelope_digest,
                evidence_digest=evidence_digest,
                receipt_dir=str(receipt_dir) if receipt_dir else None,
                stdout=prior_stdout + verify_proc.stdout,
                stderr=prior_stderr + verify_proc.stderr,
                exit_code=verify_rec["outcome_exit_code"],
                verified=False,
                errors=["no engine trust root configured; checked verdict refused"],
                findings_count=findings_total,
                revision=legacy_revision,
                raw=raw,
            )

        verdict = Verdict(engine_verdict)
        return Outcome(
            verdict=verdict,
            tier=tier,
            grade=Grade.D2,
            scope=SCOPE,
            request_digest=request_digest,
            invocation_envelope_digest=invocation_envelope_digest,
            evidence_digest=evidence_digest,
            receipt_dir=str(receipt_dir) if receipt_dir else None,
            stdout=prior_stdout + verify_proc.stdout,
            stderr=prior_stderr + verify_proc.stderr,
            exit_code=verify_rec["outcome_exit_code"],
            verified=verified,
            findings_count=findings_total,
            revision=legacy_revision,
            raw=raw,
        )

    def reverify(
        self,
        receipt_dir: str,
        *,
        expected_request: Optional[RunRequest] = None,
    ) -> OutcomeRecord:
        """Re-verify a persisted receipt using the pinned engine.

        Routed by receipt kind: a directory holding a cooperative
        receipt goes through the strict Rust cooperative reverifier
        (fresh replay + engine binding); anything else keeps the raw
        run-receipt (`verify-run`) path. Negotiated-v2 receipts require an
        independent complete request; receipt contents are never promoted
        into expected request data by Python.
        """
        if type(self) is not MultiverseRunner:
            raise TypeError("runner subclasses are not an authority boundary")
        if type(receipt_dir) is not str:
            raise TypeError("receipt_dir must be a string")
        if expected_request is not None:
            if type(expected_request) is not RunRequest:
                raise TypeError("expected_request must be a RunRequest or None")
            try:
                expected_request = _snapshot_run_request(expected_request)
            except (AttributeError, TypeError, ValueError) as exc:
                return _error_outcome(
                    [f"invalid expected request at reverify boundary: {exc}"]
                )
        receipt = Path(receipt_dir)
        try:
            _reject_symlink_components(receipt)
            if not stat.S_ISDIR(os.lstat(receipt).st_mode):
                return _error_outcome(["receipt path is not a directory"])
        except (OSError, ValueError) as exc:
            return _error_outcome([f"receipt path refused: {exc}"])

        try:
            engine_lease, private_dir = _private_engine_directory(
                "vibe-halt-reverify-"
            )
        except (OSError, ValueError):
            return _error_outcome(["private engine directory refused"])
        with engine_lease:
            return self._reverify_with_private_engine(
                receipt, private_dir, expected_request
            )

    def _reverify_with_private_engine(
        self,
        receipt: Path,
        private_dir: Path,
        expected_request: Optional[RunRequest],
    ) -> OutcomeRecord:
        try:
            engine, untrusted, copied_engine_digest = _copy_and_verify_engine(
                self._policy, private_dir
            )
        except ValueError as e:
            return _error_outcome([str(e)])

        cooperative_receipt = receipt / COOPERATIVE_RECEIPT_NAME
        run_receipt = receipt / RUN_RECEIPT_NAME
        cooperative_present = os.path.lexists(cooperative_receipt)
        run_present = os.path.lexists(run_receipt)
        if cooperative_present and run_present:
            return _error_outcome(
                ["ambiguous receipt directory contains multiple receipt kinds"],
                receipt_dir=receipt,
            )
        if cooperative_present:
            try:
                receipt_bytes = _read_bounded_regular_file(
                    cooperative_receipt, MAX_COOPERATIVE_RECEIPT_BYTES
                )
            except ValueError as exc:
                return _error_outcome(
                    [f"cooperative receipt refused: {exc}"],
                    receipt_dir=receipt,
                )
            is_v2 = receipt_bytes.startswith(
                (COOPERATIVE_RECEIPT_SCHEMA_V2 + "\n").encode("ascii")
            )
            if is_v2 and expected_request is None:
                return _v2_error_outcome(
                    [
                        "v2 cooperative receipt requires an independent "
                        "expected_request"
                    ],
                    untrusted=untrusted,
                    receipt_dir=receipt,
                )
            if is_v2 and (
                expected_request is None
                or expected_request.protocol_requirement is None
            ):
                return _v2_error_outcome(
                    ["v2 expected_request lacks negotiated protocol components"],
                    untrusted=untrusted,
                    receipt_dir=receipt,
                )
            if (
                expected_request is not None
                and expected_request.protocol_requirement is not None
            ):
                requirement = expected_request.protocol_requirement
                try:
                    manifest = _query_protocol_manifest(
                        engine, private_dir, copied_engine_digest
                    )
                except ValueError as exc:
                    return _v2_error_outcome(
                        [f"protocol manifest refused: {exc}"],
                        untrusted=untrusted,
                        receipt_dir=receipt,
                    )
                matching_descriptor = _protocol_descriptor_for(
                    manifest, requirement
                )
                features = _negotiated_feature_closure(
                    requirement, matching_descriptor
                )
                descriptor = matching_descriptor or manifest.descriptors[0]
                if matching_descriptor is not None:
                    try:
                        _validate_revision_coordinate(requirement, descriptor)
                    except ValueError as exc:
                        return _v2_error_outcome(
                            [str(exc)],
                            untrusted=untrusted,
                            receipt_dir=receipt,
                        )
                request_dict = _request_dict(expected_request)
                request_digest = _sha256_hex(
                    _canonical_json_bytes(request_dict)
                )
                return self._verify_cooperative_v2_receipt(
                    cooperative_receipt,
                    engine,
                    private_dir,
                    request=expected_request,
                    manifest=manifest,
                    descriptor=descriptor,
                    features=features,
                    copied_engine_digest=copied_engine_digest,
                    untrusted=untrusted,
                    receipt_dir=receipt,
                    request_digest=request_digest,
                )
            return self._verify_cooperative_receipt(
                cooperative_receipt,
                engine,
                private_dir,
                untrusted=untrusted,
                tier=Tier.TIER2,
                receipt_dir=receipt,
                expected_request=expected_request,
            )
        if not run_present or not _regular_file_without_links(run_receipt):
            return _error_outcome(
                ["unrecognized or invalid receipt directory"], receipt_dir=receipt
            )

        verify_proc = _invoke_engine(
            [
                str(engine),
                "verify-run",
                "--out",
                str(receipt),
                "--engine",
                str(engine),
            ],
            cwd=private_dir,
        )
        verify_rec = _parse_verify_run(verify_proc.stdout)
        verify_errors = (
            None
            if verify_rec is None
            else _validate_verify_run_record(verify_rec, verify_proc.returncode)
        )
        if verify_rec is None or verify_errors is None:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED if untrusted else Grade.D0,
                scope=SCOPE,
                receipt_dir=str(receipt),
                stdout=verify_proc.stdout,
                stderr=verify_proc.stderr,
                exit_code=verify_proc.returncode,
                verified=False,
                errors=["verify-run produced no valid machine record"],
            )

        evidence_digest = verify_rec["evidence_digest"]
        engine_verdict = verify_rec["verdict"]
        findings_total = verify_rec["findings_total"]
        verified = verify_rec["verified"]

        if not verify_rec["authentic"]:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED if untrusted else Grade.D0,
                scope=SCOPE,
                evidence_digest=evidence_digest,
                receipt_dir=str(receipt),
                stdout=verify_proc.stdout,
                stderr=verify_proc.stderr,
                exit_code=verify_rec["outcome_exit_code"],
                verified=False,
                errors=verify_errors,
                findings_count=findings_total,
                raw=verify_rec,
            )

        if (
            not untrusted
            and verify_rec["engine_sha256"] != self._policy.expected_digest
        ):
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.D0,
                scope=SCOPE,
                evidence_digest=evidence_digest,
                receipt_dir=str(receipt),
                stdout=verify_proc.stdout,
                stderr=verify_proc.stderr,
                exit_code=1,
                verified=False,
                errors=["verifier engine digest does not match the configured trust root"],
                findings_count=findings_total,
                raw=verify_rec,
            )

        if untrusted:
            return Outcome(
                verdict=Verdict.UNCHECKED,
                tier=Tier.TIER1,
                grade=Grade.UNTRUSTED,
                scope=SCOPE,
                evidence_digest=evidence_digest,
                receipt_dir=str(receipt),
                stdout=verify_proc.stdout,
                stderr=verify_proc.stderr,
                exit_code=verify_rec["outcome_exit_code"],
                verified=False,
                errors=["no engine trust root configured; checked verdict refused"],
                findings_count=findings_total,
                raw=verify_rec,
            )

        if not verified:
            return Outcome(
                verdict=Verdict.UNCHECKED,
                tier=Tier.TIER1,
                grade=Grade.D0,
                scope=SCOPE,
                evidence_digest=evidence_digest,
                receipt_dir=str(receipt),
                stdout=verify_proc.stdout,
                stderr=verify_proc.stderr,
                exit_code=verify_rec["outcome_exit_code"],
                verified=False,
                errors=["fresh reproduction remained semantically unchecked"],
                findings_count=findings_total,
                raw=verify_rec,
            )

        verdict = Verdict(engine_verdict)
        return Outcome(
            verdict=verdict,
            tier=Tier.TIER1,
            grade=Grade.D0,
            scope=SCOPE,
            evidence_digest=evidence_digest,
            receipt_dir=str(receipt),
            stdout=verify_proc.stdout,
            stderr=verify_proc.stderr,
            exit_code=verify_rec["outcome_exit_code"],
            verified=verified,
            findings_count=findings_total,
            raw=verify_rec,
        )
