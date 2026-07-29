"""Tests for the strict Python-to-Rust adapter (R0/R1)."""

import hashlib
import os
import shutil
import tempfile
import unittest
from pathlib import Path

from vibe_halt import EnginePolicy, Grade, MultiverseRunner, RunRequest, Tier, Verdict


def _engine_path() -> str:
    """Resolve the vh engine binary for this test run."""
    env = os.environ.get("VIBE_HALT_ENGINE")
    if env:
        return str(Path(env).resolve())
    # tests live at clients/python/tests; repo root is three parents up.
    default = Path(__file__).resolve().parents[3] / "target" / "debug" / "vh"
    return str(default)


def _engine_digest(path: str) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class AdapterSmokeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.engine = _engine_path()
        if not Path(cls.engine).exists():
            raise RuntimeError(
                f"vh engine not found at {cls.engine}; build it with cargo build -p vh-cli"
            )
        cls.engine_digest = _engine_digest(cls.engine)
        cls.policy = EnginePolicy(cls.engine, cls.engine_digest)

    def test_missing_engine(self):
        policy = EnginePolicy("/definitely/not/a/repository", "a" * 64)
        runner = MultiverseRunner(policy)
        outcome = runner.run(RunRequest("demo", 3))
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertTrue(any("does not exist" in e for e in outcome.errors))

    def test_engine_digest_mismatch(self):
        policy = EnginePolicy(self.engine, "0" * 64)
        runner = MultiverseRunner(policy)
        outcome = runner.run(RunRequest("demo", 3))
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertTrue(any("digest mismatch" in e for e in outcome.errors))

    def test_unsupported_workload(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("does-not-exist", 3))
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertFalse(outcome.verified)

    def test_demo_clean(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("demo", 10))
        self.assertEqual(outcome.verdict, Verdict.CLEAN)
        self.assertEqual(outcome.tier, Tier.TIER1)
        self.assertEqual(outcome.grade, Grade.D0)
        self.assertTrue(outcome.verified)
        self.assertIsNotNone(outcome.evidence_digest)
        self.assertIsNotNone(outcome.invocation_envelope_digest)
        self.assertEqual(outcome.findings_count, 0)

        rev = runner.reverify(outcome.receipt_dir)
        self.assertEqual(rev.evidence_digest, outcome.evidence_digest)
        self.assertTrue(rev.verified)
        self.assertEqual(rev.verdict, Verdict.CLEAN)

    def test_demo_buggy_findings(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("demo-buggy", 20))
        self.assertEqual(outcome.verdict, Verdict.FINDINGS)
        self.assertEqual(outcome.tier, Tier.TIER1)
        self.assertEqual(outcome.grade, Grade.D0)
        self.assertTrue(outcome.verified)
        self.assertGreater(outcome.findings_count, 0)

    def test_demo_unchecked(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("demo", 10, check_divergence=False))
        self.assertEqual(outcome.verdict, Verdict.UNCHECKED)
        self.assertTrue(outcome.verified)
        self.assertEqual(outcome.grade, Grade.D0)

    def test_two_invocations_distinct_envelope(self):
        runner = MultiverseRunner(self.policy)
        req = RunRequest("demo", 10)
        a = runner.run(req)
        b = runner.run(req)
        self.assertEqual(a.evidence_digest, b.evidence_digest)
        self.assertNotEqual(
            a.invocation_envelope_digest, b.invocation_envelope_digest
        )

    def test_no_trust_root(self):
        policy = EnginePolicy(self.engine)
        runner = MultiverseRunner(policy)
        outcome = runner.run(RunRequest("demo", 10))
        self.assertEqual(outcome.verdict, Verdict.UNCHECKED)
        self.assertEqual(outcome.grade, Grade.UNTRUSTED)
        self.assertTrue(outcome.verified)

    def test_tampered_receipt(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("demo", 10))
        self.assertTrue(outcome.verified)
        run_path = Path(outcome.receipt_dir) / "run.ndjson"
        text = run_path.read_text()
        tampered = text.replace('"verdict":"CLEAN"', '"verdict":"TAMPERED"', 1)
        run_path.write_text(tampered)

        rev = runner.reverify(outcome.receipt_dir)
        self.assertFalse(rev.verified)
        self.assertEqual(rev.verdict, Verdict.ERROR)

    def test_non_empty_output_root(self):
        root = Path(tempfile.mkdtemp(prefix="vibe-halt-nonempty-"))
        try:
            (root / "spurious").write_text("x")
            runner = MultiverseRunner(self.policy)
            outcome = runner.run(RunRequest("demo", 3, output_root=str(root)))
            self.assertEqual(outcome.verdict, Verdict.ERROR)
            self.assertTrue(any("not empty" in e for e in outcome.errors))
        finally:
            shutil.rmtree(root, ignore_errors=True)
