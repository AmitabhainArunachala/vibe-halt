"""Tests for the strict Python-to-Rust adapter (R0/R1)."""

import hashlib
import os
import shutil
import tempfile
import unittest
from unittest import mock
from pathlib import Path

from vibe_halt import EnginePolicy, Grade, MultiverseRunner, RunRequest, Tier, Verdict
import vibe_halt.core.runner as runner_module


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

    def test_uppercase_engine_digest_is_normalized_once(self):
        policy = EnginePolicy(self.engine, self.engine_digest.upper())
        self.assertEqual(policy.expected_digest, self.engine_digest)
        outcome = MultiverseRunner(policy).run(RunRequest("demo", 3))
        self.assertTrue(outcome.verified, outcome.errors)

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
        self.assertFalse(outcome.verified)
        self.assertEqual(outcome.grade, Grade.D0)
        self.assertEqual(outcome.exit_code, 3)

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
        self.assertFalse(outcome.verified)

    def test_no_trust_root_reverify_cannot_promote_generic_receipt(self):
        trusted = MultiverseRunner(self.policy)
        original = trusted.run(RunRequest("demo", 3))
        self.assertTrue(original.verified)
        untrusted = MultiverseRunner(EnginePolicy(self.engine))
        replay = untrusted.reverify(original.receipt_dir)
        self.assertEqual(replay.verdict, Verdict.UNCHECKED)
        self.assertEqual(replay.grade, Grade.UNTRUSTED)
        self.assertFalse(replay.verified)

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
            self.assertTrue(any("already exists" in e for e in outcome.errors))
        finally:
            shutil.rmtree(root, ignore_errors=True)

    def test_output_root_regular_file_refused_and_preserved(self):
        root = Path(tempfile.mkdtemp(prefix="vibe-halt-outfile-"))
        try:
            target = root / "not-a-dir"
            target.write_text("precious")
            runner = MultiverseRunner(self.policy)
            outcome = runner.run(RunRequest("demo", 3, output_root=str(target)))
            self.assertEqual(outcome.verdict, Verdict.ERROR)
            self.assertEqual(target.read_text(), "precious")
        finally:
            shutil.rmtree(root, ignore_errors=True)

    def test_output_root_symlink_refused(self):
        root = Path(tempfile.mkdtemp(prefix="vibe-halt-outlink-"))
        try:
            target = root / "target"
            target.mkdir()
            link = root / "link"
            os.symlink(target, link)
            runner = MultiverseRunner(self.policy)
            outcome = runner.run(RunRequest("demo", 3, output_root=str(link)))
            self.assertEqual(outcome.verdict, Verdict.ERROR)
            self.assertFalse((target / "run.ndjson").exists())
        finally:
            shutil.rmtree(root, ignore_errors=True)

    @unittest.skipUnless(os.name == "posix", "Unix ownership/mode contract")
    def test_untrusted_tmpdir_never_executes_or_reverifies_trusted_engine(self):
        root = Path(tempfile.mkdtemp(prefix="vibe-halt-unsafe-temp-parent-"))
        try:
            runner = MultiverseRunner(self.policy)
            receipt_root = root / "trusted-receipt"
            original = runner.run(
                RunRequest("demo", 3, output_root=str(receipt_root))
            )
            self.assertTrue(original.verified, original.errors)

            shared = root / "shared"
            shared.mkdir(mode=0o700)
            modes = [0o777]
            if os.geteuid() != 0:
                modes.append(0o1777)
            for mode in modes:
                with self.subTest(mode=oct(mode)):
                    os.chmod(shared, mode)
                    run_root = root / f"refused-{mode:o}"
                    with mock.patch.object(
                        runner_module.tempfile, "tempdir", str(shared)
                    ), mock.patch.object(runner_module, "_invoke_engine") as invoke:
                        run_outcome = runner.run(
                            RunRequest("demo", 3, output_root=str(run_root))
                        )
                        reverified = runner.reverify(original.receipt_dir)
                    invoke.assert_not_called()
                    for outcome in (run_outcome, reverified):
                        self.assertEqual(outcome.verdict, Verdict.ERROR)
                        self.assertFalse(outcome.verified)
                        self.assertIn("private engine directory refused", outcome.errors)
                    self.assertEqual(list(shared.iterdir()), [])
                    os.chmod(shared, 0o700)
        finally:
            shutil.rmtree(root, ignore_errors=True)

    def test_mutated_request_is_revalidated_before_invoke(self):
        request = RunRequest("demo", 3)
        object.__setattr__(request, "universes", True)
        runner = MultiverseRunner(self.policy)
        with mock.patch.object(runner_module, "_invoke_engine") as invoke:
            outcome = runner.run(request)
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        invoke.assert_not_called()

    def test_control_bearing_request_is_refused_before_invoke(self):
        with self.assertRaisesRegex(ValueError, "control characters"):
            RunRequest("demo", 3, source_commit="line\nbreak")
        with self.assertRaisesRegex(ValueError, "control characters"):
            RunRequest("demo", 3, source_commit="right-to-left\u202e")

        request = RunRequest("demo", 3)
        object.__setattr__(request, "source_commit", "line\nbreak")
        runner = MultiverseRunner(self.policy)
        with mock.patch.object(runner_module, "_invoke_engine") as invoke:
            outcome = runner.run(request)
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        invoke.assert_not_called()

    def test_request_subclasses_are_rejected_before_invoke(self):
        class DerivedRequest(RunRequest):
            pass

        runner = MultiverseRunner(self.policy)
        with self.assertRaises(TypeError):
            runner.run(DerivedRequest("demo", 3))

    def test_generic_request_numeric_and_receipt_profile_bounds(self):
        self.assertEqual(RunRequest("demo", 10_000).universes, 10_000)
        for kwargs in [
            {"universes": 10_001},
            {"universes": 1, "seed": -1},
            {"universes": 1, "seed": 1 << 64},
            {"universes": 1, "schedule": "pct:3"},
            {"universes": 1, "record_tape": True},
            {"universes": 1, "shrink": True},
        ]:
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                RunRequest("demo", **kwargs)

    def test_empty_source_commit_is_present_and_request_bound(self):
        outcome = MultiverseRunner(self.policy).run(
            RunRequest("demo", 3, source_commit="")
        )
        self.assertEqual(outcome.verdict, Verdict.CLEAN, outcome.errors)
        self.assertTrue(outcome.verified)

    def test_source_commit_exact_string_bound_is_accepted_and_over_bound_refused(self):
        exact = "x" * 4096
        self.assertEqual(RunRequest("demo", 1, source_commit=exact).source_commit, exact)
        with self.assertRaisesRegex(ValueError, "4096-byte request bound"):
            RunRequest("demo", 1, source_commit="x" * 4097)

    def test_generic_receipt_from_another_valid_request_cannot_be_adopted(self):
        replacement = MultiverseRunner(self.policy).run(
            RunRequest("demo", 3, seed=0xBEEF)
        )
        self.assertTrue(replacement.verified)
        target_parent = Path(tempfile.mkdtemp(prefix="vibe-halt-swap-target-"))
        target_root = target_parent / "receipt"
        original_invoke = runner_module._invoke_engine
        swapped = False

        def invoke(argv, cwd=None):
            nonlocal swapped
            completed = original_invoke(argv, cwd=cwd)
            if len(argv) > 1 and argv[1] == "run" and not swapped:
                for child in target_root.iterdir():
                    if child.is_dir():
                        shutil.rmtree(child)
                    else:
                        child.unlink()
                for child in Path(replacement.receipt_dir).iterdir():
                    destination = target_root / child.name
                    if child.is_dir():
                        shutil.copytree(child, destination)
                    else:
                        shutil.copy2(child, destination)
                swapped = True
            return completed

        with mock.patch.object(runner_module, "_invoke_engine", side_effect=invoke):
            outcome = MultiverseRunner(self.policy).run(
                RunRequest("demo", 3, output_root=str(target_root))
            )
        self.assertTrue(swapped)
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertFalse(outcome.verified)
        self.assertTrue(any("request does not match" in error for error in outcome.errors))

    def test_initial_generic_status_must_match_fresh_reverification(self):
        original_invoke = runner_module._invoke_engine
        changed = False

        def invoke(argv, cwd=None):
            nonlocal changed
            completed = original_invoke(argv, cwd=cwd)
            if len(argv) > 1 and argv[1] == "run" and not changed:
                changed = True
                return __import__("subprocess").CompletedProcess(
                    completed.args, 3, completed.stdout, completed.stderr
                )
            return completed

        with mock.patch.object(runner_module, "_invoke_engine", side_effect=invoke):
            outcome = MultiverseRunner(self.policy).run(RunRequest("demo", 3))
        self.assertEqual(outcome.verdict, Verdict.ERROR)
        self.assertFalse(outcome.verified)
        self.assertTrue(any("status does not match" in error for error in outcome.errors))
