"""Tests for the R2 cooperative D2 transport through the Python adapter."""

import hashlib
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from vibe_halt import EnginePolicy, Grade, MultiverseRunner, RunRequest, Tier, Verdict
import vibe_halt.core.runner as runner_module


def _engine_path() -> str:
    env = os.environ.get("VIBE_HALT_ENGINE")
    if env:
        return str(Path(env).resolve())
    default = Path(__file__).resolve().parents[3] / "target" / "debug" / "vh"
    return str(default)


def _engine_digest(path: str) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class CooperativeTransportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.engine = _engine_path()
        if not Path(cls.engine).exists():
            raise RuntimeError(
                f"vh engine not found at {cls.engine}; build it with cargo build -p vh-cli"
            )
        cls.engine_digest = _engine_digest(cls.engine)
        cls.policy = EnginePolicy(cls.engine, cls.engine_digest)

    def test_cooperative_echo_clean(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(
            RunRequest("cooperative-echo", 1, transport="cooperative")
        )
        self.assertEqual(outcome.verdict, Verdict.CLEAN)
        self.assertEqual(outcome.tier, Tier.TIER2)
        self.assertEqual(outcome.grade, Grade.D2)
        self.assertTrue(outcome.verified)
        self.assertEqual(outcome.findings_count, 0)
        self.assertIsNotNone(outcome.evidence_digest)
        self.assertIsNotNone(outcome.invocation_envelope_digest)

    def test_cooperative_no_trust_root(self):
        policy = EnginePolicy(self.engine)
        runner = MultiverseRunner(policy)
        outcome = runner.run(
            RunRequest("cooperative-echo", 1, transport="cooperative")
        )
        self.assertEqual(outcome.verdict, Verdict.UNCHECKED)
        self.assertEqual(outcome.grade, Grade.UNTRUSTED)
        self.assertFalse(outcome.verified)

    def test_non_cooperative_request_unchanged(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("demo", 10))
        self.assertEqual(outcome.verdict, Verdict.CLEAN)
        self.assertEqual(outcome.tier, Tier.TIER1)
        self.assertEqual(outcome.grade, Grade.D0)


class CooperativeRequestContractTests(unittest.TestCase):
    """Item 7/8: the canonical request contract rejects anything outside
    the cooperative contract before any runner invocation."""

    def test_cassette_path_requires_cooperative_transport(self):
        with self.assertRaises(ValueError):
            RunRequest(
                "cooperative-echo",
                1,
                cassette_path="/tmp/cassette.vhc",
            )

    def test_cassette_path_with_cooperative_transport_accepted(self):
        req = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            cassette_path="/tmp/cassette.vhc",
        )
        self.assertEqual(req.cassette_path, "/tmp/cassette.vhc")

    def test_cooperative_controls_table(self):
        base = dict(workload="cooperative-echo", universes=1, transport="cooperative")
        mutations = [
            ("workload", {"workload": "demo"}),
            ("universes", {"universes": 2}),
            ("seed", {"seed": 0xD1CF}),
            ("palette", {"palette": "v1"}),
            ("schedule", {"schedule": "pct"}),
            ("check_divergence", {"check_divergence": False}),
            ("record_tape", {"record_tape": True}),
            ("shrink", {"shrink": True}),
            ("source_commit", {"source_commit": "deadbeef"}),
        ]
        for name, override in mutations:
            with self.subTest(control=name):
                kwargs = dict(base)
                kwargs.update(override)
                with self.assertRaises(ValueError):
                    RunRequest(**kwargs)

    def test_cooperative_controls_require_exact_python_types(self):
        base = dict(workload="cooperative-echo", universes=1, transport="cooperative")
        mutations = [
            ("workload", b"cooperative-echo"),
            ("universes", True),
            ("seed", float(0xD1CE)),
            ("palette", b"v0"),
            ("schedule", b"fifo"),
            ("check_divergence", 1),
            ("record_tape", 0),
            ("shrink", 0),
            ("source_commit", 0),
            ("transport", b"cooperative"),
        ]
        for field, value in mutations:
            with self.subTest(field=field):
                kwargs = dict(base)
                kwargs[field] = value
                with self.assertRaises(TypeError):
                    RunRequest(**kwargs)

    def test_cooperative_canonical_request_accepted(self):
        req = RunRequest("cooperative-echo", 1, transport="cooperative")
        self.assertEqual(req.transport, "cooperative")


class CooperativeRedactionTests(unittest.TestCase):
    """Item 9: attacker content in a malformed cassette must never reach
    stdout, stderr, receipt bytes, or Python exceptions."""

    @classmethod
    def setUpClass(cls):
        cls.engine = _engine_path()
        if not Path(cls.engine).exists():
            raise RuntimeError(
                f"vh engine not found at {cls.engine}; build it with cargo build -p vh-cli"
            )
        cls.engine_digest = _engine_digest(cls.engine)
        cls.policy = EnginePolicy(cls.engine, cls.engine_digest)

    def test_malformed_cassette_redacts_attacker_content(self):
        import tempfile

        sentinel = "S3CR3T-SENTINEL-PR57"
        tmp = Path(tempfile.mkdtemp(prefix="vibe-halt-redact-"))
        cassette = tmp / "bad.vhc"
        cassette.write_text(f"{sentinel} garbage head\n")
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(
            RunRequest(
                "cooperative-echo",
                1,
                transport="cooperative",
                cassette_path=str(cassette),
            )
        )
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        blob = outcome.stdout + outcome.stderr + "".join(outcome.errors)
        self.assertNotIn(sentinel, blob)
        if outcome.receipt_dir:
            receipt = Path(outcome.receipt_dir)
            if receipt.exists():
                for entry in receipt.rglob("*"):
                    if entry.is_file():
                        self.assertNotIn(sentinel, entry.read_bytes().decode("utf-8", "replace"))


def _timeout_cassette_bytes() -> bytes:
    """Cassette file bytes: one fixture request recorded as Timeout."""

    def field(tag: bytes, value: bytes) -> bytes:
        return tag + b" " + str(len(value)).encode() + b":" + value + b"\n"

    req = b"vh-llm-request-v2\n"
    req += field(b"provider", b"fixture")
    req += field(b"model", b"cooperative-echo")
    req += b"messages 1\n"
    req += field(b"role", b"user")
    req += field(b"content", b"hello")
    req += b"tools 0\n"
    req += b"tool-choice absent\n"
    req += b"structured-output absent\n"
    req += b"params 1\n"
    req += field(b"param-key", b"temperature")
    req += field(b"param-value", b"0")
    resp = b"timeout\n"
    return b"vh-cassette-v2 1\n" + field(b"request", req) + field(b"response", resp)


class CooperativeOracleFindingTests(unittest.TestCase):
    """Item 3: the cassette Timeout finding flows through the declared
    oracle with CLI/Python parity."""

    @classmethod
    def setUpClass(cls):
        cls.engine = _engine_path()
        if not Path(cls.engine).exists():
            raise RuntimeError(
                f"vh engine not found at {cls.engine}; build it with cargo build -p vh-cli"
            )
        cls.engine_digest = _engine_digest(cls.engine)
        cls.policy = EnginePolicy(cls.engine, cls.engine_digest)

    def test_timeout_finding_matches_cli(self):
        import json
        import subprocess
        import tempfile

        tmp = Path(tempfile.mkdtemp(prefix="vibe-halt-timeout-"))
        cassette = tmp / "timeout.vhc"
        cassette.write_bytes(_timeout_cassette_bytes())

        runner = MultiverseRunner(self.policy)
        outcome = runner.run(
            RunRequest(
                "cooperative-echo",
                1,
                transport="cooperative",
                cassette_path=str(cassette),
            )
        )
        self.assertEqual(outcome.verdict, Verdict.FINDINGS)
        self.assertTrue(outcome.verified)
        self.assertEqual(outcome.findings_count, 1)
        self.assertEqual(outcome.exit_code, 1)
        self.assertEqual(outcome.raw.get("outcome_exit_code"), 1)
        self.assertEqual(
            outcome.raw.get("finding_identity"),
            "cooperative-llm-call-completed:timeout",
        )

        cli = subprocess.run(
            [self.engine, "cooperative", "--cassette", str(cassette)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(cli.returncode, 1, cli.stderr)
        cli_rec = json.loads(cli.stdout.strip().splitlines()[-1])
        self.assertEqual(cli_rec["verdict"], "FINDINGS")
        self.assertTrue(cli_rec["verified"])
        self.assertEqual(cli_rec["findings_count"], 1)
        self.assertEqual(
            cli_rec["finding_identity"], outcome.raw["finding_identity"]
        )
        self.assertEqual(cli_rec["evidence_digest"], outcome.evidence_digest)


def _success_cassette_bytes(n_entries: int, status: int = 200) -> bytes:
    """Cassette file bytes: the fixture request recorded n times as Success."""

    def field(tag: bytes, value: bytes) -> bytes:
        return tag + b" " + str(len(value)).encode() + b":" + value + b"\n"

    req = b"vh-llm-request-v2\n"
    req += field(b"provider", b"fixture")
    req += field(b"model", b"cooperative-echo")
    req += b"messages 1\n"
    req += field(b"role", b"user")
    req += field(b"content", b"hello")
    req += b"tools 0\n"
    req += b"tool-choice absent\n"
    req += b"structured-output absent\n"
    req += b"params 1\n"
    req += field(b"param-key", b"temperature")
    req += field(b"param-value", b"0")
    resp = b"success %d\n" % status + field(b"body", b"cooperative-reply\n")
    out = b"vh-cassette-v2 %d\n" % n_entries
    for _ in range(n_entries):
        out += field(b"request", req) + field(b"response", resp)
    return out


class CooperativeReverifyTests(unittest.TestCase):
    """Item 4: reverify() routes by receipt kind through the strict Rust
    cooperative reverifier; item 1: trust ordering."""

    @classmethod
    def setUpClass(cls):
        cls.engine = _engine_path()
        if not Path(cls.engine).exists():
            raise RuntimeError(
                f"vh engine not found at {cls.engine}; build it with cargo build -p vh-cli"
            )
        cls.engine_digest = _engine_digest(cls.engine)
        cls.policy = EnginePolicy(cls.engine, cls.engine_digest)

    def test_reverify_routes_cooperative_receipt(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("cooperative-echo", 1, transport="cooperative"))
        self.assertEqual(outcome.verdict, Verdict.CLEAN)
        rev = runner.reverify(outcome.receipt_dir)
        self.assertTrue(rev.verified)
        self.assertEqual(rev.verdict, Verdict.CLEAN)
        self.assertEqual(rev.tier, Tier.TIER2)
        self.assertEqual(rev.grade, Grade.D2)
        self.assertEqual(rev.evidence_digest, outcome.evidence_digest)

    def test_tampered_cooperative_receipt_fails_closed(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("cooperative-echo", 1, transport="cooperative"))
        receipt = Path(outcome.receipt_dir) / "cooperative.receipt"
        data = receipt.read_bytes()
        receipt.write_bytes(data[: len(data) // 2])
        rev = runner.reverify(outcome.receipt_dir)
        self.assertFalse(rev.verified)
        self.assertEqual(rev.verdict, Verdict.ERROR)

    def test_receipt_directory_symlink_and_ambiguous_kinds_are_refused(self):
        import tempfile

        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("cooperative-echo", 1, transport="cooperative"))
        link = Path(tempfile.mkdtemp(prefix="vibe-halt-route-link-")) / "receipt-link"
        os.symlink(outcome.receipt_dir, link)
        linked = runner.reverify(str(link))
        self.assertEqual(linked.verdict, Verdict.ERROR)
        self.assertFalse(linked.verified)

        run_marker = Path(outcome.receipt_dir) / "run.ndjson"
        run_marker.write_text("do-not-touch")
        ambiguous = runner.reverify(outcome.receipt_dir)
        self.assertEqual(ambiguous.verdict, Verdict.ERROR)
        self.assertIn("ambiguous", " ".join(ambiguous.errors))
        self.assertEqual(run_marker.read_text(), "do-not-touch")

    def test_valid_receipt_cannot_be_substituted_for_another_request(self):
        import tempfile

        runner = MultiverseRunner(self.policy)
        clean = runner.run(RunRequest("cooperative-echo", 1, transport="cooperative"))
        tmp = Path(tempfile.mkdtemp(prefix="vibe-halt-context-bind-"))
        timeout = tmp / "timeout.vhc"
        timeout.write_bytes(_timeout_cassette_bytes())
        expected = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            cassette_path=str(timeout),
        )
        substituted = runner._verify_cooperative_receipt(
            Path(clean.receipt_dir) / "cooperative.receipt",
            Path(self.engine),
            tmp,
            untrusted=False,
            tier=Tier.TIER2,
            receipt_dir=Path(clean.receipt_dir),
            expected_request=expected,
        )
        self.assertEqual(substituted.verdict, Verdict.ERROR)
        self.assertFalse(substituted.verified)
        self.assertTrue(
            any("expected-cassette-mismatch" in error for error in substituted.errors),
            substituted.errors,
        )

    def test_authentic_unchecked_evidence_matches_cli_with_trust_root(self):
        # An authentic, freshly reproduced Rust UNCHECKED outcome remains
        # UNCHECKED in Python. It is not a checked success, and its transport
        # taint diagnostics must survive unchanged.
        import json
        import subprocess
        import tempfile

        tmp = Path(tempfile.mkdtemp(prefix="vibe-halt-tainted-"))
        cassette = tmp / "unconsumed.vhc"
        cassette.write_bytes(_success_cassette_bytes(2))
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(
            RunRequest(
                "cooperative-echo",
                1,
                transport="cooperative",
                cassette_path=str(cassette),
            )
        )
        self.assertEqual(outcome.verdict, Verdict.UNCHECKED)
        self.assertEqual(outcome.grade, Grade.D2)
        self.assertEqual(outcome.exit_code, 3)
        self.assertFalse(outcome.verified)
        self.assertTrue(
            any("taint" in e for e in outcome.errors),
            f"Rust transport error must be intact: {outcome.errors}",
        )

        cli = subprocess.run(
            [self.engine, "cooperative", "--cassette", str(cassette)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(cli.returncode, 3, cli.stderr)
        cli_rec = json.loads(cli.stdout.strip().splitlines()[-1])
        self.assertEqual(cli_rec["verdict"], outcome.verdict.value)
        self.assertEqual(cli_rec["evidence_digest"], outcome.evidence_digest)
        self.assertEqual(cli_rec["result_digest"], outcome.raw["result_digest"])
        self.assertEqual(cli_rec["receipt_sha256"], outcome.raw["receipt_sha256"])
        self.assertFalse(cli_rec["verified"])

    def test_authentic_unchecked_evidence_without_trust_root_stays_untrusted(self):
        import tempfile

        tmp = Path(tempfile.mkdtemp(prefix="vibe-halt-tainted-untrusted-"))
        cassette = tmp / "unconsumed.vhc"
        cassette.write_bytes(_success_cassette_bytes(2))
        outcome = MultiverseRunner(EnginePolicy(self.engine)).run(
            RunRequest(
                "cooperative-echo",
                1,
                transport="cooperative",
                cassette_path=str(cassette),
            )
        )
        self.assertEqual(outcome.verdict, Verdict.UNCHECKED)
        self.assertEqual(outcome.grade, Grade.UNTRUSTED)
        self.assertEqual(outcome.exit_code, 3)
        self.assertFalse(outcome.verified)
        self.assertTrue(
            any("taint" in e for e in outcome.errors),
            f"Rust transport error must be intact: {outcome.errors}",
        )

    def test_authentic_unchecked_preserves_empty_rust_errors(self):
        import json
        import subprocess
        import tempfile

        tmp = Path(tempfile.mkdtemp(prefix="vibe-halt-unsupported-success-"))
        cassette = tmp / "status-201.vhc"
        cassette.write_bytes(_success_cassette_bytes(1, status=201))
        outcome = MultiverseRunner(self.policy).run(
            RunRequest(
                "cooperative-echo",
                1,
                transport="cooperative",
                cassette_path=str(cassette),
            )
        )
        self.assertEqual(outcome.verdict, Verdict.UNCHECKED)
        self.assertEqual(outcome.grade, Grade.D2)
        self.assertEqual(outcome.exit_code, 3)
        self.assertFalse(outcome.verified)
        self.assertEqual(outcome.errors, [])

        cli = subprocess.run(
            [self.engine, "cooperative", "--cassette", str(cassette)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(cli.returncode, 3, cli.stderr)
        cli_rec = json.loads(cli.stdout.strip().splitlines()[-1])
        self.assertEqual(cli_rec["verdict"], outcome.verdict.value)
        self.assertEqual(json.loads(cli_rec["errors"]), outcome.errors)
        self.assertEqual(cli_rec["evidence_digest"], outcome.evidence_digest)
        self.assertEqual(cli_rec["result_digest"], outcome.raw["result_digest"])
        self.assertEqual(cli_rec["receipt_sha256"], outcome.raw["receipt_sha256"])


class CooperativeCrossSurfaceTests(unittest.TestCase):
    """Group 7: cross-surface concurrency and CLI/Python parity."""

    @classmethod
    def setUpClass(cls):
        cls.engine = _engine_path()
        if not Path(cls.engine).exists():
            raise RuntimeError(
                f"vh engine not found at {cls.engine}; build it with cargo build -p vh-cli"
            )
        cls.engine_digest = _engine_digest(cls.engine)
        cls.policy = EnginePolicy(cls.engine, cls.engine_digest)

    def test_concurrent_runs_isolated_identical_identity(self):
        import tempfile
        import threading

        runner = MultiverseRunner(self.policy)
        results: dict = {}
        barrier = threading.Barrier(2)
        parent = Path(tempfile.mkdtemp(prefix="vibe-halt-concurrent-"))

        def work(name: str):
            barrier.wait()
            results[name] = runner.run(
                RunRequest(
                    "cooperative-echo",
                    1,
                    transport="cooperative",
                    output_root=str(parent / name),
                )
            )

        threads = [
            threading.Thread(target=work, args=("a",)),
            threading.Thread(target=work, args=("b",)),
        ]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        self.assertEqual(results["a"].verdict, Verdict.CLEAN, results["a"].errors)
        self.assertEqual(results["b"].verdict, Verdict.CLEAN, results["b"].errors)
        self.assertEqual(results["a"].evidence_digest, results["b"].evidence_digest)
        self.assertNotEqual(
            results["a"].receipt_dir, results["b"].receipt_dir
        )

    def test_competing_python_runs_admit_at_most_one_without_deletion(self):
        import tempfile
        import threading

        runner = MultiverseRunner(self.policy)
        parent = Path(tempfile.mkdtemp(prefix="vibe-halt-contested-"))
        out = parent / "shared"
        sentinel = parent / "sentinel"
        sentinel.write_text("preserve")
        barrier = threading.Barrier(2)
        results: list = []

        def work():
            barrier.wait()
            results.append(
                runner.run(
                    RunRequest(
                        "cooperative-echo",
                        1,
                        transport="cooperative",
                        output_root=str(out),
                    )
                )
            )

        threads = [threading.Thread(target=work), threading.Thread(target=work)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        self.assertEqual(sum(result.verdict == Verdict.CLEAN for result in results), 1)
        winner = next(result for result in results if result.verdict == Verdict.CLEAN)
        loser = next(result for result in results if result.verdict != Verdict.CLEAN)
        self.assertEqual(loser.verdict, Verdict.ERROR)
        self.assertFalse(loser.verified)
        self.assertIsNone(loser.evidence_digest)
        self.assertNotEqual(loser.evidence_digest, winner.evidence_digest)
        self.assertEqual(sentinel.read_text(), "preserve")
        self.assertTrue((out / "cooperative.receipt").is_file())

    def test_clean_evidence_digest_matches_cli(self):
        import json
        import subprocess
        import tempfile

        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("cooperative-echo", 1, transport="cooperative"))
        self.assertEqual(outcome.verdict, Verdict.CLEAN)
        cli_out = Path(tempfile.mkdtemp(prefix="vibe-halt-cli-parity-parent-")) / "out"
        cli = subprocess.run(
            [self.engine, "cooperative", "--out", str(cli_out)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(cli.returncode, 0, cli.stderr)
        cli_rec = json.loads(cli.stdout.strip().splitlines()[-1])
        self.assertEqual(cli_rec["verdict"], "CLEAN")
        self.assertEqual(cli_rec["evidence_digest"], outcome.evidence_digest)
        self.assertEqual(cli_rec["result_digest"], outcome.raw.get("result_digest"))
        self.assertEqual(cli_rec["receipt_sha256"], outcome.raw.get("receipt_sha256"))
        self.assertEqual(
            hashlib.sha256((cli_out / "cooperative.receipt").read_bytes()).hexdigest(),
            cli_rec["receipt_sha256"],
        )

    def test_verifier_record_parser_and_types_fail_closed(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("cooperative-echo", 1, transport="cooperative"))
        record = {
            key: outcome.raw[key] for key in runner_module._COOPERATIVE_VERIFY_FIELDS
        }
        self.assertEqual(
            runner_module._validate_cooperative_verify_record(
                record, 0, "cooperative-echo"
            ),
            [],
        )
        mutations = [
            ("verified", 1),
            ("authentic", 1),
            ("findings_count", False),
            ("verdict", "MYSTERY"),
            ("exit_code", 1),
            ("outcome_exit_code", 1),
        ]
        for field, value in mutations:
            with self.subTest(field=field):
                candidate = dict(record)
                candidate[field] = value
                self.assertIsNone(
                    runner_module._validate_cooperative_verify_record(
                        candidate, 0, "cooperative-echo"
                    )
                )
        extra = dict(record)
        extra["unknown"] = "field"
        self.assertIsNone(
            runner_module._validate_cooperative_verify_record(
                extra, 0, "cooperative-echo"
            )
        )

        encoded = __import__("json").dumps(record, separators=(",", ":"))
        self.assertIsNone(
            runner_module._parse_cooperative_verify(encoded + "\n" + encoded)
        )
        duplicate = (
            '{"record":"cooperative-verify",'
            '"record":"cooperative-verify",'
            '"schema":"vh-cooperative-verify-v1"}'
        )
        self.assertIsNone(runner_module._parse_cooperative_verify(duplicate))
        self.assertIsNone(runner_module._parse_cooperative_verify("[]"))
        self.assertIsNone(
            runner_module._parse_cooperative_verify(encoded + "\ntrailing-garbage")
        )
        self.assertIsNone(
            runner_module._parse_cooperative_verify("[" * 2000 + "]" * 2000)
        )
        self.assertIsNone(runner_module._strict_errors('["\\ud800"]'))
        self.assertIsNone(
            runner_module._strict_errors(__import__("json").dumps(["line\n\x1b[31m"]))
        )
        self.assertIsNone(
            runner_module._strict_errors(__import__("json").dumps(["right-to-left\u202e"]))
        )
        self.assertEqual(
            runner_module._strict_errors(
                __import__("json").dumps([""] * 64, separators=(",", ":"))
            ),
            [""] * 64,
        )
        self.assertIsNone(
            runner_module._strict_errors(__import__("json").dumps([""] * 65))
        )
        for padded in ("\n" + encoded, " " + encoded, encoded + "\n\n", encoded + "\r\n"):
            with self.subTest(padded=repr(padded[:20])):
                self.assertIsNone(runner_module._parse_cooperative_verify(padded))

    def test_engine_output_overflow_is_an_explicit_invocation_failure(self):
        import subprocess

        valid_prefix = b'{"record":"cooperative-verify","schema":"vh-cooperative-verify-v1"}\n'

        def flood(argv, **kwargs):
            kwargs["stdout"].write(
                valid_prefix + b"x" * runner_module.MAX_ENGINE_OUTPUT_BYTES
            )
            return subprocess.CompletedProcess(argv, 0)

        with mock.patch.object(runner_module.subprocess, "run", side_effect=flood):
            completed = runner_module._invoke_engine(["/not/executed"])
        self.assertEqual(completed.returncode, 125)
        self.assertIn("bounded capture", completed.stderr)
        self.assertIsNone(
            runner_module._parse_cooperative_verify(completed.stdout)
        )


class CooperativeNegotiationV2ContractTests(unittest.TestCase):
    """Issue #90 red contract: negotiated operation, features, and revision."""

    TARGET_REVISION = "abbbaf8284752607e8a80324c87e39302848c4fca50a5ad034ca40562a38d60a"

    @classmethod
    def setUpClass(cls):
        cls.engine = _engine_path()
        if not Path(cls.engine).exists():
            raise RuntimeError(
                f"vh engine not found at {cls.engine}; build it with cargo build -p vh-cli"
            )
        cls.engine_digest = _engine_digest(cls.engine)
        cls.policy = EnginePolicy(cls.engine, cls.engine_digest)

    @staticmethod
    def _requirement(
        digest="abbbaf8284752607e8a80324c87e39302848c4fca50a5ad034ca40562a38d60a",
        required_features=(),
    ):
        from vibe_halt import (
            OperationId,
            ProtocolRequirement,
            RequestedTargetRevision,
        )

        return ProtocolRequirement(
            operation=OperationId("cooperative-target", 1),
            required_features=required_features,
            requested_target_revision=RequestedTargetRevision(
                "cooperative-child-source-v1", "sha256", digest
            ),
        )

    def test_protocol_requirement_is_immutable_and_canonical(self):
        from dataclasses import FrozenInstanceError
        from vibe_halt import FeatureId, OperationId, ProtocolRequirement

        requirement = self._requirement()
        with self.assertRaises(FrozenInstanceError):
            requirement.operation = OperationId("other", 1)
        with self.assertRaises(ValueError):
            ProtocolRequirement(
                operation=OperationId("cooperative-target", 1),
                required_features=(
                    FeatureId("fresh-replay", 1),
                    FeatureId("cooperative-cassette", 2),
                ),
                requested_target_revision=requirement.requested_target_revision,
            )
        with self.assertRaises(ValueError):
            RunRequest("demo", 1, invocation_id="x" * 129)
        with self.assertRaises(ValueError):
            ProtocolRequirement(
                operation=OperationId("cooperative-target", 1),
                required_features=(FeatureId("fresh-replay", 1),) * 2,
                requested_target_revision=requirement.requested_target_revision,
            )

    def test_public_result_constructors_cannot_mint_trust_positive_data(self):
        from dataclasses import replace
        from vibe_halt import Outcome, RevisionReport

        safe = Outcome(
            verdict=Verdict.ERROR,
            tier=Tier.UNKNOWN,
            grade=Grade.UNTRUSTED,
            scope="caller-data",
        )
        for changes in (
            {"verdict": Verdict.CLEAN},
            {"verdict": Verdict.FINDINGS},
            {"grade": Grade.D0},
            {"grade": Grade.D2},
            {"verified": True},
        ):
            with self.subTest(outcome_changes=changes), self.assertRaises(TypeError):
                replace(safe, **changes)
        with self.assertRaises(TypeError):
            class ForgedOutcome(Outcome):
                pass

        legacy = RevisionReport(
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
        for changes in (
            {"binding": "bound"},
            {"fresh_observed": "0" * 64},
            {"verified_observed": "0" * 64},
        ):
            with self.subTest(revision_changes=changes), self.assertRaises(TypeError):
                replace(legacy, **changes)
        with self.assertRaises(TypeError):
            class ForgedRevision(RevisionReport):
                pass

    def test_hostile_nested_protocol_mutations_fail_before_output_reservation(self):
        mutations = (
            ("operation", object()),
            ("required_features", []),
            ("required_features", (object(),)),
            ("requested_target_revision", object()),
        )
        for field, hostile_value in mutations:
            with self.subTest(field=field, value_type=type(hostile_value).__name__):
                requirement = self._requirement()
                request = RunRequest(
                    "cooperative-echo",
                    1,
                    transport="cooperative",
                    protocol_requirement=requirement,
                )
                object.__setattr__(requirement, field, hostile_value)
                with mock.patch.object(
                    runner_module, "_prepare_output_root"
                ) as prepare_output_root:
                    outcome = MultiverseRunner(self.policy).run(request)
                self.assertEqual(outcome.verdict, Verdict.ERROR)
                self.assertFalse(outcome.verified)
                self.assertIn("invalid request at run boundary", outcome.errors[0])
                prepare_output_root.assert_not_called()

    def test_hostile_nested_protocol_mutation_fails_before_reverify_io(self):
        requirement = self._requirement()
        expected = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=requirement,
        )
        object.__setattr__(requirement, "operation", object())
        with mock.patch.object(
            runner_module, "_copy_and_verify_engine"
        ) as engine_snapshot:
            outcome = MultiverseRunner(self.policy).reverify(
                "/not/inspected", expected_request=expected
            )
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertIn("invalid expected request", outcome.errors[0])
        engine_snapshot.assert_not_called()

    def test_manifest_query_uses_actual_private_copy_and_parser_rejects_mutations(self):
        lease, private_dir = runner_module._private_engine_directory(
            "vibe-halt-manifest-test-"
        )
        with lease:
            copied, untrusted, copied_digest = runner_module._copy_and_verify_engine(
                self.policy, private_dir
            )
            self.assertFalse(untrusted)
            manifest = runner_module._query_protocol_manifest(
                copied, private_dir, copied_digest
            )
            self.assertEqual(manifest.engine_sha256, copied_digest)
            process = runner_module._invoke_engine_bytes(
                [str(copied), "protocol-manifest"], cwd=private_dir
            )
            self.assertEqual(process.returncode, 0, process.stderr)
            encoded = process.stdout

        marker = b"manifest-id 64:"
        manifest_start = encoded.index(marker) + len(marker)
        bad_manifest_id = (
            encoded[:manifest_start]
            + b"0" * 64
            + encoded[manifest_start + 64 :]
        )
        mutations = {
            "missing-field": encoded.replace(b"optional-features 0\n", b"", 1),
            "duplicate-field": encoded.replace(
                b"descriptors 1\n", b"descriptors 1\ndescriptors 1\n", 1
            ),
            "reordered-field": encoded.replace(
                b"request-schema 25:vh-cooperative-request-v2\n"
                b"outcome-schema 25:vh-cooperative-outcome-v2\n",
                b"outcome-schema 25:vh-cooperative-outcome-v2\n"
                b"request-schema 25:vh-cooperative-request-v2\n",
                1,
            ),
            "truncated": encoded[:-1],
            "trailing-data": encoded + b"x",
            "noncanonical-count": encoded.replace(
                b"descriptors 1\n", b"descriptors 01\n", 1
            ),
            "noncanonical-length": encoded.replace(
                b"operation 21:", b"operation 021:", 1
            ),
            "manifest-identity": bad_manifest_id,
        }
        for name, value in mutations.items():
            with self.subTest(name=name), self.assertRaises(ValueError):
                runner_module._parse_protocol_manifest(value)

    def test_legacy_client_request_digest_preimages_remain_pinned(self):
        cases = (
            (
                RunRequest("demo", 3),
                "3e90575a72fc9c334334916f442a1642211ecba785d67065e1d5130fc3f056a3",
            ),
            (
                RunRequest(
                    "cooperative-echo", 1, transport="cooperative"
                ),
                "f886d50c0c56b1176546fed113108b3c95f0c072c32bc7f2d037da35d7b4318c",
            ),
        )
        for request, expected in cases:
            with self.subTest(workload=request.workload):
                actual = runner_module._sha256_hex(
                    runner_module._canonical_json_bytes(
                        runner_module._request_dict(request)
                    )
                )
                self.assertEqual(actual, expected)

    def test_manifest_engine_digest_mismatch_is_local_error_before_v2_dispatch(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(),
        )
        canonical = runner_module._invoke_engine_bytes(
            [self.engine, "protocol-manifest"]
        )
        self.assertEqual(canonical.returncode, 0, canonical.stderr)
        manifest = runner_module._parse_protocol_manifest(canonical.stdout)
        descriptor = manifest.descriptors[0]
        mismatched_engine_digest = (
            "0" * 64 if manifest.engine_sha256 != "0" * 64 else "1" * 64
        )
        preimage = bytearray(
            (runner_module._PROTOCOL_MANIFEST_ID_DOMAIN + "\n").encode("ascii")
        )
        for tag, value in (
            ("schema", runner_module.PROTOCOL_MANIFEST_SCHEMA),
            ("engine-sha256", mismatched_engine_digest),
            ("operation", descriptor.operation),
            ("request-schema", descriptor.request_schema),
            ("outcome-schema", descriptor.outcome_schema),
            ("receipt-schema", descriptor.receipt_schema),
            ("verifier-schema", descriptor.verifier_schema),
            ("observation-subject", descriptor.observation_subject),
            ("revision-algorithm", descriptor.revision_algorithm),
            ("revision-policy", descriptor.revision_policy),
            ("execution-binding", descriptor.execution_binding),
            (
                "observation-to-exec-channel",
                descriptor.observation_to_exec_channel,
            ),
        ):
            preimage.extend(runner_module._wire_frame(tag, value.encode("ascii")))
        preimage.extend(
            f"mandatory-features {len(descriptor.mandatory_features)}\n".encode(
                "ascii"
            )
        )
        for feature in descriptor.mandatory_features:
            preimage.extend(
                runner_module._wire_frame("feature", feature.encode("ascii"))
            )
        preimage.extend(
            f"optional-features {len(descriptor.optional_features)}\n".encode(
                "ascii"
            )
        )
        for feature in descriptor.optional_features:
            preimage.extend(
                runner_module._wire_frame("feature", feature.encode("ascii"))
            )
        mismatched_manifest_id = hashlib.sha256(bytes(preimage)).hexdigest()
        mismatched_record = canonical.stdout.replace(
            manifest.engine_sha256.encode("ascii"),
            mismatched_engine_digest.encode("ascii"),
            1,
        ).replace(
            manifest.manifest_id.encode("ascii"),
            mismatched_manifest_id.encode("ascii"),
            1,
        )
        parsed_mismatch = runner_module._parse_protocol_manifest(mismatched_record)
        self.assertEqual(parsed_mismatch.engine_sha256, mismatched_engine_digest)
        self.assertEqual(parsed_mismatch.manifest_id, mismatched_manifest_id)
        forged_process = mock.Mock(
            returncode=0,
            stdout=mismatched_record,
            stderr=b"",
        )
        with mock.patch.object(
            runner_module,
            "_invoke_engine_bytes",
            return_value=forged_process,
        ), mock.patch.object(
            MultiverseRunner, "_run_cooperative_v2"
        ) as v2_dispatch:
            outcome = MultiverseRunner(self.policy).run(request)
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertFalse(outcome.verified)
        self.assertIn("engine digest differs", outcome.errors[0])
        v2_dispatch.assert_not_called()

    def test_empty_caller_extras_still_send_full_mandatory_closure(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(),
        )
        outcome = MultiverseRunner(self.policy).run(request)
        self.assertEqual(outcome.verdict, Verdict.CLEAN, outcome.errors)
        self.assertTrue(outcome.verified)
        self.assertIsNotNone(outcome.protocol)
        self.assertIsNotNone(outcome.revision)
        self.assertEqual(outcome.raw["operation"], "cooperative-target-v1")
        self.assertEqual(
            outcome.raw["features"],
            "cooperative-cassette-v2,fresh-replay-v1,observed-child-source-sha256-v1",
        )
        self.assertEqual(outcome.raw["revision_binding"], "bound")
        self.assertEqual(outcome.raw["execution_binding"], "staged-d2")
        self.assertEqual(outcome.raw["observation_to_exec_channel"], "open")
        self.assertEqual(outcome.raw["claimed_observed_revision"], outcome.raw["fresh_observed_revision"])
        self.assertNotEqual(outcome.request_digest, outcome.raw["engine_request_id"])
        self.assertEqual(
            outcome.revision.observation_subject,
            "cooperative-child-source-v1",
        )
        self.assertEqual(outcome.revision.revision_algorithm, "sha256")
        self.assertEqual(outcome.revision.revision_policy, "bound-required")
        self.assertEqual(
            outcome.to_dict()["revision"]["observation_subject"],
            "cooperative-child-source-v1",
        )

    def test_unknown_caller_extra_reaches_typed_engine_refusal(self):
        from vibe_halt import FeatureId

        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(
                required_features=(FeatureId("unknown-capability", 1),)
            ),
        )
        outcome = MultiverseRunner(self.policy).run(request)
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertFalse(outcome.verified)
        self.assertEqual(outcome.raw["refusal"], "unsupported-feature")
        self.assertEqual(outcome.raw["executions"], 0)
        self.assertIsNotNone(outcome.refusal)
        self.assertEqual(outcome.refusal.reason, "unsupported-feature")

    def test_unknown_operation_reaches_typed_engine_refusal(self):
        from vibe_halt import OperationId, ProtocolRequirement

        base = self._requirement()
        requirement = ProtocolRequirement(
            operation=OperationId("unknown-operation", 1),
            required_features=(),
            requested_target_revision=base.requested_target_revision,
        )
        outcome = MultiverseRunner(self.policy).run(
            RunRequest(
                "cooperative-echo",
                1,
                transport="cooperative",
                protocol_requirement=requirement,
            )
        )
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertFalse(outcome.verified)
        self.assertEqual(outcome.raw["refusal"], "unsupported-operation")
        self.assertEqual(outcome.raw["executions"], 0)

    def test_no_trust_root_cannot_promote_bound_engine_data(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(),
        )
        outcome = MultiverseRunner(EnginePolicy(self.engine)).run(request)
        self.assertEqual(outcome.verdict, Verdict.UNCHECKED, outcome.errors)
        self.assertEqual(outcome.grade, Grade.UNTRUSTED)
        self.assertFalse(outcome.verified)
        self.assertTrue(outcome.raw["verified"])
        self.assertEqual(outcome.raw["revision_binding"], "bound")
        self.assertEqual(
            outcome.revision.verified_observed,
            outcome.raw["verified_observed_revision"],
        )

    def test_requested_revision_mismatch_maps_typed_zero_execution_refusal(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement("0" * 64),
        )
        outcome = MultiverseRunner(self.policy).run(request)
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertFalse(outcome.verified)
        self.assertEqual(outcome.raw["refusal"], "requested-revision-mismatch")
        self.assertEqual(outcome.raw["executions"], 0)
        self.assertFalse((Path(outcome.receipt_dir) / "cooperative.receipt").exists())

    def test_revision_coordinate_mismatch_refuses_before_v2_execution(self):
        from vibe_halt import ProtocolRequirement, RequestedTargetRevision

        cases = (
            ("other-target-v1", "sha256"),
            ("cooperative-child-source-v1", "sha512"),
        )
        for subject, algorithm in cases:
            with self.subTest(subject=subject, algorithm=algorithm):
                base = self._requirement()
                request = RunRequest(
                    "cooperative-echo",
                    1,
                    transport="cooperative",
                    protocol_requirement=ProtocolRequirement(
                        operation=base.operation,
                        required_features=(),
                        requested_target_revision=RequestedTargetRevision(
                            subject, algorithm, self.TARGET_REVISION
                        ),
                    ),
                )
                with mock.patch.object(
                    runner_module, "_build_cooperative_v2_args"
                ) as execution_args:
                    outcome = MultiverseRunner(self.policy).run(request)
                self.assertEqual(outcome.verdict, Verdict.ERROR)
                self.assertFalse(outcome.verified)
                self.assertIn("coordinate differs", outcome.errors[0])
                execution_args.assert_not_called()
                self.assertFalse(
                    (Path(outcome.receipt_dir) / "cooperative.receipt").exists()
                )

    def test_unknown_revision_refuses_bound_required_before_execution(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(None),
        )
        outcome = MultiverseRunner(self.policy).run(request)
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertFalse(outcome.verified)
        self.assertEqual(outcome.raw["refusal"], "requested-revision-mismatch")
        self.assertEqual(outcome.raw["executions"], 0)
        self.assertFalse((Path(outcome.receipt_dir) / "cooperative.receipt").exists())

    def test_v2_reverify_requires_independent_expected_request(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(),
        )
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(request)
        self.assertEqual(outcome.verdict, Verdict.CLEAN, outcome.errors)
        without_expected = runner.reverify(outcome.receipt_dir)
        self.assertEqual(without_expected.verdict, Verdict.ERROR)
        with_expected = runner.reverify(outcome.receipt_dir, expected_request=request)
        self.assertEqual(with_expected.verdict, Verdict.CLEAN, with_expected.errors)
        self.assertTrue(with_expected.verified)

    def test_v2_reverify_rejects_alternate_expected_revision(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(),
        )
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(request)
        self.assertEqual(outcome.verdict, Verdict.CLEAN, outcome.errors)
        alternate = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement("0" * 64),
        )
        replay = runner.reverify(
            outcome.receipt_dir, expected_request=alternate
        )
        self.assertEqual(replay.verdict, Verdict.ERROR)
        self.assertFalse(replay.verified)
        self.assertEqual(
            replay.raw["schema"],
            runner_module.COOPERATIVE_VERIFY_FAILURE_SCHEMA_V1,
        )
        self.assertEqual(
            replay.raw["verification_failure"], "expected-request-mismatch"
        )
        self.assertEqual(replay.raw["executions"], 0)
        self.assertFalse(replay.raw["authentic"])

    def test_direct_cli_and_python_v2_records_have_field_parity(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(),
        )
        manifest_process = runner_module._invoke_engine_bytes(
            [self.engine, "protocol-manifest"]
        )
        self.assertEqual(manifest_process.returncode, 0, manifest_process.stderr)
        manifest = runner_module._parse_protocol_manifest(manifest_process.stdout)
        descriptor = manifest.descriptors[0]
        features = descriptor.mandatory_features
        with tempfile.TemporaryDirectory(prefix="vh-python-parity-") as root:
            direct_out = Path(root) / "direct"
            direct_out.mkdir(mode=0o700)
            direct_process = runner_module._invoke_engine_bytes(
                [self.engine]
                + runner_module._build_cooperative_v2_args(
                    request, direct_out, manifest, features
                )
            )
        self.assertEqual(direct_process.returncode, 0, direct_process.stderr)
        direct = runner_module._parse_v2_machine_record(
            direct_process.stdout, runner_module.COOPERATIVE_OUTCOME_SCHEMA_V2
        )
        python = MultiverseRunner(self.policy).run(request)
        self.assertEqual(python.verdict, Verdict.CLEAN, python.errors)
        for field in (
            "manifest_id",
            "engine_request_id",
            "evidence_id",
            "receipt_sha256",
            "operation",
            "features_tuple",
            "claimed_observed_revision",
            "fresh_observed_revision",
            "verified_observed_revision",
            "execution_binding",
            "observation_to_exec_channel",
            "verdict",
        ):
            with self.subTest(field=field):
                self.assertEqual(python.raw[field], direct[field])
        self.assertEqual(direct["executions"], 4)
        self.assertEqual(python.raw["executions"], 2)
        self.assertNotEqual(python.request_digest, direct["engine_request_id"])

    def test_nonempty_output_root_refuses_before_engine_snapshot(self):
        with tempfile.TemporaryDirectory(prefix="vh-python-occupied-") as root:
            output = Path(root) / "output"
            output.mkdir(mode=0o700)
            (output / "occupied").write_bytes(b"occupied")
            request = RunRequest(
                "cooperative-echo",
                1,
                transport="cooperative",
                output_root=str(output),
                protocol_requirement=self._requirement(),
            )
            with mock.patch.object(
                runner_module, "_copy_and_verify_engine"
            ) as engine_snapshot:
                outcome = MultiverseRunner(self.policy).run(request)
            self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertIn("already exists", outcome.errors[0])
        engine_snapshot.assert_not_called()
        self.assertFalse((output / "cooperative.receipt").exists())

    def test_preexisting_empty_output_root_refuses_before_engine_snapshot(self):
        with tempfile.TemporaryDirectory(prefix="vh-python-preexisting-") as root:
            output = Path(root) / "output"
            output.mkdir(mode=0o700)
            request = RunRequest(
                "cooperative-echo",
                1,
                transport="cooperative",
                output_root=str(output),
                protocol_requirement=self._requirement(),
            )
            with mock.patch.object(
                runner_module, "_copy_and_verify_engine"
            ) as engine_snapshot:
                outcome = MultiverseRunner(self.policy).run(request)
            self.assertEqual(outcome.verdict, Verdict.ERROR)
            self.assertIn("already exists", outcome.errors[0])
            engine_snapshot.assert_not_called()
            self.assertEqual(list(output.iterdir()), [])

    def test_two_v2_runs_reserve_isolated_output_roots(self):
        request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=self._requirement(),
        )
        runner = MultiverseRunner(self.policy)
        first = runner.run(request)
        second = runner.run(request)
        self.assertEqual(first.verdict, Verdict.CLEAN, first.errors)
        self.assertEqual(second.verdict, Verdict.CLEAN, second.errors)
        self.assertNotEqual(first.receipt_dir, second.receipt_dir)
        self.assertTrue(
            (Path(first.receipt_dir) / "cooperative.receipt").is_file()
        )
        self.assertTrue(
            (Path(second.receipt_dir) / "cooperative.receipt").is_file()
        )

    def test_legacy_v1_is_explicitly_unbound(self):
        outcome = MultiverseRunner(self.policy).run(
            RunRequest("cooperative-echo", 1, transport="cooperative")
        )
        self.assertEqual(outcome.verdict, Verdict.CLEAN, outcome.errors)
        self.assertIsNotNone(outcome.revision)
        self.assertEqual(outcome.revision.binding, "legacy-unbound")
        self.assertIsNone(outcome.revision.observation_subject)
        self.assertIsNone(outcome.revision.revision_algorithm)
        self.assertIsNone(outcome.revision.revision_policy)
        self.assertIsNone(outcome.revision.requested)
        self.assertIsNone(outcome.revision.claimed_observed)
        self.assertIsNone(outcome.revision.fresh_observed)
        self.assertIsNone(outcome.revision.verified_observed)
        self.assertEqual(outcome.raw["revision_binding"], "legacy-unbound")
