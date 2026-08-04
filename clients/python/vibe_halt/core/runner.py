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
from dataclasses import asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .request import EnginePolicy, RunRequest
from .result import Grade, Outcome, Tier, Verdict


SCOPE = "vibe-halt.run.v0"
VERIFY_RUN_SCHEMA = "vh-verify-run-v2"
VERIFY_COOPERATIVE_SCHEMA = "vh-cooperative-verify-v1"
COOPERATIVE_RECEIPT_NAME = "cooperative.receipt"
RUN_RECEIPT_NAME = "run.ndjson"
MAX_ENGINE_BYTES = 128 << 20
MAX_ENGINE_OUTPUT_BYTES = 1 << 20
ENGINE_INVOCATION_TIMEOUT_SECONDS = 120
MAX_DIAGNOSTIC_BYTES = 256
MAX_DIAGNOSTIC_ITEMS = 64
_CANONICAL_JSON_SEPARATORS = (",", ":")
_GENERIC_ENGINE_REQUEST_DOMAIN = "vh-generic-engine-request-v1"


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


def _copy_and_verify_engine(policy: EnginePolicy, private_dir: Path) -> Tuple[Path, bool]:
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
    return dest, untrusted


def _read_bounded_process_stream(handle) -> Tuple[str, bool]:
    handle.seek(0)
    data = handle.read(MAX_ENGINE_OUTPUT_BYTES + 1)
    truncated = len(data) > MAX_ENGINE_OUTPUT_BYTES
    data = data[:MAX_ENGINE_OUTPUT_BYTES]
    text = data.decode("utf-8", errors="replace")
    return text, truncated


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


def _prepare_output_root(request: RunRequest) -> Path:
    if request.output_root is not None:
        raw = Path(request.output_root)
        try:
            _reject_symlink_components(raw, allow_missing_leaf=True)
            _validate_cross_uid_safe_directory(raw.parent)
            try:
                mode = os.lstat(raw).st_mode
            except FileNotFoundError:
                # Only the final component may be absent. `os.mkdir` is an
                # exclusive leaf reservation and never follows/makes parents.
                os.mkdir(raw, mode=0o700)
                mode = os.lstat(raw).st_mode
            if not stat.S_ISDIR(mode):
                raise ValueError("output root is not a directory")
            with os.scandir(raw) as entries:
                if next(entries, None) is not None:
                    raise ValueError("output root is not empty")
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


def _error_outcome(errors: List[str], *, receipt_dir: Optional[Path] = None) -> Outcome:
    return Outcome(
        verdict=Verdict.ERROR,
        tier=Tier.UNKNOWN,
        grade=Grade.UNTRUSTED,
        scope=SCOPE,
        errors=list(errors),
        receipt_dir=str(receipt_dir) if receipt_dir is not None else None,
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

    def run(self, request: RunRequest) -> Outcome:
        if type(self) is not MultiverseRunner:
            raise TypeError("runner subclasses are not an authority boundary")
        if type(request) is not RunRequest:
            raise TypeError("request must be a RunRequest instance")

        # Frozen dataclasses can still be mutated through low-level Python
        # mechanisms. Reconstruct one exact base-class instance at the entry
        # boundary, which reruns strict type/contract validation, and use only
        # that immutable snapshot for hashing and argv construction.
        try:
            request = RunRequest(**asdict(request))
        except (TypeError, ValueError) as exc:
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
    ) -> Outcome:
        try:
            engine, untrusted = _copy_and_verify_engine(self._policy, engine_dir)
        except ValueError as e:
            return _error_outcome([str(e)], receipt_dir=out_dir)

        invocation_id = request.invocation_id or secrets.token_hex(16)
        request_dict: Dict[str, Any] = asdict(request)
        request_dict.pop("output_root", None)
        request_dict.pop("invocation_id", None)
        request_digest = _sha256_hex(_canonical_json_bytes(request_dict))

        if request.transport == "cooperative":
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
    ) -> Outcome:
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
    ) -> Outcome:
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
                raw=verify_rec,
            )

        evidence_digest = verify_rec["evidence_digest"]
        result_digest = verify_rec["result_digest"]
        engine_verdict = verify_rec["verdict"]
        findings_total = verify_rec["findings_count"]
        verified = verify_rec["verified"]

        raw: Dict[str, Any] = dict(verify_rec)
        raw["coop_stdout"] = prior_stdout
        raw["coop_stderr"] = prior_stderr
        raw["verify_stderr"] = verify_proc.stderr

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
            raw=raw,
        )

    def reverify(self, receipt_dir: str) -> Outcome:
        """Re-verify a persisted receipt using the pinned engine.

        Routed by receipt kind: a directory holding a cooperative
        receipt goes through the strict Rust cooperative reverifier
        (fresh replay + engine binding); anything else keeps the raw
        run-receipt (`verify-run`) path.
        """
        if type(self) is not MultiverseRunner:
            raise TypeError("runner subclasses are not an authority boundary")
        if type(receipt_dir) is not str:
            raise TypeError("receipt_dir must be a string")
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
            return self._reverify_with_private_engine(receipt, private_dir)

    def _reverify_with_private_engine(
        self, receipt: Path, private_dir: Path
    ) -> Outcome:
        try:
            engine, untrusted = _copy_and_verify_engine(self._policy, private_dir)
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
            if not _regular_file_without_links(cooperative_receipt):
                return _error_outcome(
                    ["cooperative receipt is not a regular no-link file"],
                    receipt_dir=receipt,
                )
            return self._verify_cooperative_receipt(
                cooperative_receipt,
                engine,
                private_dir,
                untrusted=untrusted,
                tier=Tier.TIER2,
                receipt_dir=receipt,
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
