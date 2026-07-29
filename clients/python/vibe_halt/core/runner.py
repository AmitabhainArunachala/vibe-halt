"""Strict Python-to-Rust adapter: one runner method + reverify."""

from __future__ import annotations

import hashlib
import json
import os
import secrets
import shutil
import subprocess
import tempfile
from dataclasses import asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .request import EnginePolicy, RunRequest
from .result import Grade, Outcome, Tier, Verdict


SCOPE = "vibe-halt.run.v0"
VERIFY_RUN_SCHEMA = "vh-verify-run-v1"
_CANONICAL_JSON_SEPARATORS = (",", ":")


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


def _copy_and_verify_engine(policy: EnginePolicy, private_dir: Path) -> Tuple[Path, bool]:
    """Copy the engine into `private_dir` and verify it when a trust root exists.

    Returns the path to run and a flag that is True when no trust root was
    configured."""
    source = Path(policy.path)
    if not source.is_file():
        raise ValueError(f"engine path does not exist: {policy.path!r}")
    dest = private_dir / ".vibe-halt-engine"
    shutil.copy2(source, dest)
    os.chmod(dest, 0o755)
    untrusted = policy.expected_digest is None
    if not untrusted:
        actual = _sha256_hex(dest.read_bytes())
        if actual != policy.expected_digest:
            raise ValueError(
                f"engine digest mismatch: expected {policy.expected_digest}, got {actual}"
            )
    return dest, untrusted


def _prepare_output_root(request: RunRequest) -> Path:
    if request.output_root is not None:
        root = Path(request.output_root).resolve()
        if root.exists():
            if any(root.iterdir()):
                raise ValueError(f"output root is not empty: {root}")
        else:
            root.mkdir(parents=True, exist_ok=False)
        return root
    return Path(tempfile.mkdtemp(prefix="vibe-halt-run-"))


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
    if request.source_commit:
        args.extend(["--source-commit", request.source_commit])
    return args


def _parse_verify_run(stdout: str) -> Optional[Dict[str, Any]]:
    for line in reversed(stdout.splitlines()):
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        if rec.get("record") == "verify-run" and rec.get("schema") == VERIFY_RUN_SCHEMA:
            return rec
    return None


def _decode_errors(errors_field: Any) -> List[str]:
    if isinstance(errors_field, list):
        return [str(e) for e in errors_field]
    if isinstance(errors_field, str):
        try:
            decoded = json.loads(errors_field)
            if isinstance(decoded, list):
                return [str(e) for e in decoded]
        except json.JSONDecodeError:
            pass
        return [errors_field]
    return []


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

    Python is never a second simulator: every verdict is the Rust
    engine's, verified by the same Rust binary, and bound to an
    invocation envelope the Python side only hashes, never mints.
    """

    def __init__(self, policy: EnginePolicy):
        self._policy = policy

    def _invoke(
        self, argv: List[str], cwd: Optional[Path] = None
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            argv,
            cwd=cwd,
            capture_output=True,
            text=True,
            shell=False,
            env={"PATH": os.environ.get("PATH", "")},
        )

    def run(self, request: RunRequest) -> Outcome:
        if not isinstance(request, RunRequest):
            raise TypeError("request must be a RunRequest instance")

        try:
            out_dir = _prepare_output_root(request)
        except ValueError as e:
            return _error_outcome([str(e)])

        engine_dir = Path(tempfile.mkdtemp(prefix="vibe-halt-engine-"))
        try:
            engine, untrusted = _copy_and_verify_engine(self._policy, engine_dir)
        except ValueError as e:
            return _error_outcome([str(e)], receipt_dir=out_dir)

        invocation_id = request.invocation_id or secrets.token_hex(16)
        request_dict: Dict[str, Any] = asdict(request)
        request_dict.pop("output_root", None)
        request_dict.pop("invocation_id", None)
        request_digest = _sha256_hex(_canonical_json_bytes(request_dict))

        run_proc = self._invoke(
            [str(engine)] + _build_run_args(request, out_dir), cwd=engine_dir
        )
        if run_proc.returncode == 2 and not (out_dir / "run.ndjson").exists():
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
                errors=["engine run failed before writing a receipt"],
            )

        verify_proc = self._invoke(
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
        if verify_rec is None:
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
        findings_total = int(verify_rec.get("findings_total", 0))
        verified = bool(verify_rec.get("verified"))

        raw: Dict[str, Any] = dict(verify_rec)
        raw["run_stdout"] = run_proc.stdout
        raw["run_stderr"] = run_proc.stderr
        raw["run_exit_code"] = run_proc.returncode

        if not verified:
            errors = _decode_errors(verify_rec.get("errors", "[]"))
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
                errors=errors,
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
                verified=True,
                errors=["no engine trust root configured; checked verdict refused"],
                findings_count=findings_total,
                raw=raw,
            )

        try:
            verdict = Verdict(engine_verdict)
        except ValueError:
            verdict = Verdict.UNCHECKED
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
            exit_code=run_proc.returncode,
            verified=True,
            findings_count=findings_total,
            raw=raw,
        )

    def reverify(self, receipt_dir: str) -> Outcome:
        """Re-verify a raw run receipt using the pinned engine."""
        private_dir = Path(tempfile.mkdtemp(prefix="vibe-halt-reverify-"))
        try:
            engine, _ = _copy_and_verify_engine(self._policy, private_dir)
        except ValueError as e:
            return _error_outcome([str(e)])

        receipt = Path(receipt_dir).resolve()
        verify_proc = self._invoke(
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
        if verify_rec is None:
            return Outcome(
                verdict=Verdict.ERROR,
                tier=Tier.TIER1,
                grade=Grade.D0,
                scope=SCOPE,
                receipt_dir=str(receipt),
                stdout=verify_proc.stdout,
                stderr=verify_proc.stderr,
                exit_code=verify_proc.returncode,
                verified=False,
                errors=["verify-run produced no valid machine record"],
            )

        evidence_digest = verify_rec.get("evidence_digest")
        engine_verdict = verify_rec.get("verdict", "UNCHECKED")
        findings_total = int(verify_rec.get("findings_total", 0))
        verified = bool(verify_rec.get("verified"))

        if not verified:
            errors = _decode_errors(verify_rec.get("errors", "[]"))
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
                errors=errors,
                findings_count=findings_total,
                raw=verify_rec,
            )

        try:
            verdict = Verdict(engine_verdict)
        except ValueError:
            verdict = Verdict.UNCHECKED
        return Outcome(
            verdict=verdict,
            tier=Tier.TIER1,
            grade=Grade.D0,
            scope=SCOPE,
            evidence_digest=evidence_digest,
            invocation_envelope_digest=verify_rec.get("result_digest"),
            receipt_dir=str(receipt),
            stdout=verify_proc.stdout,
            stderr=verify_proc.stderr,
            exit_code=0,
            verified=True,
            findings_count=findings_total,
            raw=verify_rec,
        )
