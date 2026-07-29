"""Tests for the R2 cooperative D2 transport through the Python adapter."""

import hashlib
import os
from pathlib import Path
import unittest

from vibe_halt import EnginePolicy, Grade, MultiverseRunner, RunRequest, Tier, Verdict


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
        self.assertTrue(outcome.verified)

    def test_non_cooperative_request_unchanged(self):
        runner = MultiverseRunner(self.policy)
        outcome = runner.run(RunRequest("demo", 10))
        self.assertEqual(outcome.verdict, Verdict.CLEAN)
        self.assertEqual(outcome.tier, Tier.TIER1)
        self.assertEqual(outcome.grade, Grade.D0)
