"""Tests for the R2 cooperative D2 transport through the Python adapter."""

import hashlib
import os
from pathlib import Path
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
        out.mkdir(mode=0o700)
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
            runner_module._strict_errors(__import__("json").dumps([""] * 64)),
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
