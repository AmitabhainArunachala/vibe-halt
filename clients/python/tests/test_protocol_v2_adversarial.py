"""Adversarial coverage for the negotiated-v2 positional machine records."""

import hashlib
import os
from pathlib import Path
import tempfile
import unittest

from vibe_halt import (
    EnginePolicy,
    FeatureId,
    OperationId,
    ProtocolRequirement,
    RequestedTargetRevision,
    RunRequest,
)
import vibe_halt.core.runner as runner_module


_TARGET_REVISION = "abbbaf8284752607e8a80324c87e39302848c4fca50a5ad034ca40562a38d60a"


def _engine_path() -> str:
    configured = os.environ.get("VIBE_HALT_ENGINE")
    if configured:
        return str(Path(configured).resolve())
    return str(Path(__file__).resolve().parents[3] / "target" / "debug" / "vh")


def _line(data: bytes, prefix: bytes):
    start = data.index(prefix)
    end = data.index(b"\n", start) + 1
    return start, end, data[start:end]


def _without_line(data: bytes, prefix: bytes) -> bytes:
    start, end, _ = _line(data, prefix)
    return data[:start] + data[end:]


def _with_duplicate_line(data: bytes, prefix: bytes) -> bytes:
    start, _, value = _line(data, prefix)
    return data[:start] + value + data[start:]


def _with_unknown_line(data: bytes) -> bytes:
    schema_end = data.index(b"\n") + 1
    return data[:schema_end] + b"unknown-field 1:x\n" + data[schema_end:]


def _with_swapped_adjacent_lines(
    data: bytes, first_prefix: bytes, second_prefix: bytes
) -> bytes:
    first_start, first_end, first = _line(data, first_prefix)
    second_start, second_end, second = _line(data, second_prefix)
    if first_end != second_start:
        raise AssertionError("test mutation requires adjacent fields")
    return data[:first_start] + second + first + data[second_end:]


def _with_noncanonical_frame_length(data: bytes, prefix: bytes) -> bytes:
    start, end, value = _line(data, prefix)
    tag_end = value.index(b" ") + 1
    colon = value.index(b":", tag_end)
    mutated = value[:tag_end] + b"0" + value[tag_end:colon] + value[colon:]
    return data[:start] + mutated + data[end:]


def _oversized(data: bytes) -> bytes:
    padding = runner_module.MAX_PROTOCOL_RECORD_BYTES - len(data) + 1
    return data + b"x" * padding


def _replace_line(data: bytes, prefix: bytes, replacement: bytes) -> bytes:
    start, end, _ = _line(data, prefix)
    return data[:start] + replacement + b"\n" + data[end:]


def _manifest_with_semantic_overrides(manifest, descriptor, **overrides) -> bytes:
    values = {
        "operation": descriptor.operation,
        "request-schema": descriptor.request_schema,
        "outcome-schema": descriptor.outcome_schema,
        "receipt-schema": descriptor.receipt_schema,
        "verifier-schema": descriptor.verifier_schema,
        "observation-subject": descriptor.observation_subject,
        "revision-algorithm": descriptor.revision_algorithm,
        "revision-policy": descriptor.revision_policy,
        "execution-binding": descriptor.execution_binding,
        "observation-to-exec-channel": descriptor.observation_to_exec_channel,
    }
    values.update(overrides)
    preimage = bytearray(
        (runner_module._PROTOCOL_MANIFEST_ID_DOMAIN + "\n").encode("ascii")
    )
    preimage.extend(
        runner_module._wire_frame(
            "schema", runner_module.PROTOCOL_MANIFEST_SCHEMA.encode("ascii")
        )
    )
    preimage.extend(
        runner_module._wire_frame(
            "engine-sha256", manifest.engine_sha256.encode("ascii")
        )
    )
    for tag, value in values.items():
        preimage.extend(runner_module._wire_frame(tag, value.encode("ascii")))
    preimage.extend(
        f"mandatory-features {len(descriptor.mandatory_features)}\n".encode("ascii")
    )
    for feature in descriptor.mandatory_features:
        preimage.extend(runner_module._wire_frame("feature", feature.encode("ascii")))
    preimage.extend(
        f"optional-features {len(descriptor.optional_features)}\n".encode("ascii")
    )
    for feature in descriptor.optional_features:
        preimage.extend(runner_module._wire_frame("feature", feature.encode("ascii")))
    manifest_id = hashlib.sha256(bytes(preimage)).hexdigest()

    record = bytearray((runner_module.PROTOCOL_MANIFEST_SCHEMA + "\n").encode("ascii"))
    record.extend(
        runner_module._wire_frame(
            "engine-sha256", manifest.engine_sha256.encode("ascii")
        )
    )
    record.extend(runner_module._wire_frame("manifest-id", manifest_id.encode("ascii")))
    record.extend(b"descriptors 1\n")
    for tag, value in values.items():
        record.extend(runner_module._wire_frame(tag, value.encode("ascii")))
    record.extend(
        f"mandatory-features {len(descriptor.mandatory_features)}\n".encode("ascii")
    )
    for feature in descriptor.mandatory_features:
        record.extend(runner_module._wire_frame("feature", feature.encode("ascii")))
    record.extend(
        f"optional-features {len(descriptor.optional_features)}\n".encode("ascii")
    )
    for feature in descriptor.optional_features:
        record.extend(runner_module._wire_frame("feature", feature.encode("ascii")))
    return bytes(record)


class ProtocolV2AdversarialTests(unittest.TestCase):
    """Every malformed positional record remains unusable as positive data."""

    @classmethod
    def setUpClass(cls):
        engine = Path(_engine_path())
        if not engine.exists():
            raise RuntimeError(
                f"vh engine not found at {engine}; build it with cargo build -p vh-cli"
            )
        policy = EnginePolicy(
            str(engine), hashlib.sha256(engine.read_bytes()).hexdigest()
        )
        cls.engine_lease, cls.engine_dir = runner_module._private_engine_directory(
            "vibe-halt-v2-adversarial-engine-"
        )
        cls.engine, untrusted, cls.copied_digest = (
            runner_module._copy_and_verify_engine(policy, cls.engine_dir)
        )
        if untrusted:
            raise AssertionError("test engine copy unexpectedly lacks a trust root")

        manifest_process = runner_module._invoke_engine_bytes(
            [str(cls.engine), "protocol-manifest"], cwd=cls.engine_dir
        )
        if manifest_process.returncode != 0 or manifest_process.stderr:
            raise AssertionError(
                f"canonical manifest failed: {manifest_process.stderr!r}"
            )
        cls.manifest_bytes = manifest_process.stdout
        cls.manifest = runner_module._parse_protocol_manifest(cls.manifest_bytes)
        if cls.manifest.engine_sha256 != cls.copied_digest:
            raise AssertionError("canonical manifest is not bound to the copied engine")
        cls.descriptor = cls.manifest.descriptors[0]
        cls.features = cls.descriptor.mandatory_features

        cls.requirement = ProtocolRequirement(
            operation=OperationId("cooperative-target", 1),
            required_features=(),
            requested_target_revision=RequestedTargetRevision(
                "cooperative-child-source-v1", "sha256", _TARGET_REVISION
            ),
        )
        cls.request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=cls.requirement,
        )
        cls.output_lease = tempfile.TemporaryDirectory(
            prefix="vibe-halt-v2-adversarial-output-"
        )
        cls.output_dir = Path(cls.output_lease.name) / "out"
        outcome_process = runner_module._invoke_engine_bytes(
            [str(cls.engine)]
            + runner_module._build_cooperative_v2_args(
                cls.request, cls.output_dir, cls.manifest, cls.features
            ),
            cwd=cls.engine_dir,
        )
        if outcome_process.returncode != 0 or outcome_process.stderr:
            raise AssertionError(
                f"canonical v2 outcome failed: {outcome_process.stderr!r}"
            )
        cls.outcome_bytes = outcome_process.stdout

        receipt = cls.output_dir / runner_module.COOPERATIVE_RECEIPT_NAME
        verify_process = runner_module._invoke_engine_bytes(
            [str(cls.engine)]
            + runner_module._build_verify_cooperative_v2_args(
                cls.request,
                receipt,
                cls.manifest,
                cls.descriptor,
                cls.features,
            ),
            cwd=cls.engine_dir,
        )
        if verify_process.returncode != 0 or verify_process.stderr:
            raise AssertionError(
                f"canonical v2 verify failed: {verify_process.stderr!r}"
            )
        cls.verify_bytes = verify_process.stdout

        alternate_requirement = ProtocolRequirement(
            operation=OperationId("cooperative-target", 1),
            required_features=(),
            requested_target_revision=RequestedTargetRevision(
                "cooperative-child-source-v1", "sha256", "0" * 64
            ),
        )
        alternate_request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=alternate_requirement,
        )
        failure_process = runner_module._invoke_engine_bytes(
            [str(cls.engine)]
            + runner_module._build_verify_cooperative_v2_args(
                alternate_request,
                receipt,
                cls.manifest,
                cls.descriptor,
                cls.features,
            ),
            cwd=cls.engine_dir,
        )
        if failure_process.returncode != 1 or failure_process.stderr:
            raise AssertionError(
                f"canonical v2 failure failed: {failure_process.stderr!r}"
            )
        cls.failure_bytes = failure_process.stdout

        refusal_requirement = ProtocolRequirement(
            operation=OperationId("cooperative-target", 1),
            required_features=(FeatureId("unsupported-capability", 1),),
            requested_target_revision=RequestedTargetRevision(
                "cooperative-child-source-v1", "sha256", _TARGET_REVISION
            ),
        )
        refusal_request = RunRequest(
            "cooperative-echo",
            1,
            transport="cooperative",
            protocol_requirement=refusal_requirement,
        )
        refusal_features = tuple(
            sorted((*cls.features, "unsupported-capability-v1"))
        )
        refusal_process = runner_module._invoke_engine_bytes(
            [str(cls.engine)]
            + runner_module._build_cooperative_v2_args(
                refusal_request,
                Path(cls.output_lease.name) / "refused",
                cls.manifest,
                refusal_features,
            ),
            cwd=cls.engine_dir,
        )
        if refusal_process.returncode != 4 or refusal_process.stderr:
            raise AssertionError(
                f"canonical v2 refusal failed: {refusal_process.stderr!r}"
            )
        cls.refusal_bytes = refusal_process.stdout

    @classmethod
    def tearDownClass(cls):
        cls.output_lease.cleanup()
        cls.engine_lease.cleanup()

    def _assert_all_rejected(self, parser, mutations):
        for name, value in mutations.items():
            with self.subTest(mutation=name):
                with self.assertRaises(ValueError):
                    parser(value)

    def test_real_engine_canonical_records_parse_and_validate(self):
        manifest = runner_module._parse_protocol_manifest(self.manifest_bytes)
        refusal = runner_module._parse_engine_refusal(self.refusal_bytes)
        failure = runner_module._parse_v2_verification_failure(self.failure_bytes)
        outcome = runner_module._parse_v2_machine_record(
            self.outcome_bytes, runner_module.COOPERATIVE_OUTCOME_SCHEMA_V2
        )
        verify = runner_module._parse_v2_machine_record(
            self.verify_bytes, runner_module.COOPERATIVE_VERIFY_SCHEMA_V2
        )
        self.assertEqual(manifest.engine_sha256, self.copied_digest)
        self.assertEqual(refusal.reason, "unsupported-feature")
        self.assertEqual(failure.reason, "expected-request-mismatch")
        self.assertEqual(failure.executions, 0)
        self.assertFalse(failure.authentic)
        self.assertFalse(failure.verified)
        for record, schema in (
            (outcome, runner_module.COOPERATIVE_OUTCOME_SCHEMA_V2),
            (verify, runner_module.COOPERATIVE_VERIFY_SCHEMA_V2),
        ):
            runner_module._validate_v2_machine_record(
                record,
                expected_schema=schema,
                process_returncode=0,
                manifest=self.manifest,
                descriptor=self.descriptor,
                features=self.features,
                requested_revision=f"sha256:{_TARGET_REVISION}",
            )
            self.assertEqual(record["verdict"], "CLEAN")
            self.assertTrue(record["verified"])

    def test_manifest_parser_rejects_structural_mutation_table(self):
        data = self.manifest_bytes
        mutations = {
            "oversized": _oversized(data),
            "missing": _without_line(data, b"operation "),
            "duplicate": _with_duplicate_line(data, b"operation "),
            "unknown": _with_unknown_line(data),
            "reordered": _with_swapped_adjacent_lines(
                data, b"request-schema ", b"outcome-schema "
            ),
            "truncated": data[:-1],
            "noncanonical-frame-length": _with_noncanonical_frame_length(
                data, b"operation "
            ),
            "trailing": data + b"trailing",
        }
        self._assert_all_rejected(runner_module._parse_protocol_manifest, mutations)

    def test_manifest_v1_rejects_rehashed_authority_semantic_drift(self):
        drifts = {
            "caller-trusted-policy": {"revision-policy": "caller-trusted"},
            "causal-execution": {"execution-binding": "causally-bound"},
            "closed-channel": {"observation-to-exec-channel": "closed"},
            "alternate-verifier": {"verifier-schema": "vh-cooperative-verify-v3"},
        }
        for name, overrides in drifts.items():
            with self.subTest(drift=name), self.assertRaisesRegex(
                ValueError, "unsupported by manifest v1"
            ):
                runner_module._parse_protocol_manifest(
                    _manifest_with_semantic_overrides(
                        self.manifest, self.descriptor, **overrides
                    )
                )

    def test_refusal_parser_rejects_structural_mutation_table(self):
        data = self.refusal_bytes
        mutations = {
            "oversized": _oversized(data),
            "missing": _without_line(data, b"reason "),
            "duplicate": _with_duplicate_line(data, b"reason "),
            "unknown": _with_unknown_line(data),
            "reordered": _with_swapped_adjacent_lines(
                data, b"reason ", b"engine-sha256 "
            ),
            "truncated": data[:-1],
            "noncanonical-frame-length": _with_noncanonical_frame_length(
                data, b"reason "
            ),
            "trailing": data + b"trailing",
            "unknown-reason": data.replace(
                b"reason 19:unsupported-feature\n",
                b"reason 14:invented-error\n",
                1,
            ),
        }
        self._assert_all_rejected(runner_module._parse_engine_refusal, mutations)

    def _machine_record_mutations(self, data: bytes):
        return {
            "oversized": _oversized(data),
            "missing": _without_line(data, b"operation "),
            "duplicate": _with_duplicate_line(data, b"operation "),
            "unknown": _with_unknown_line(data),
            "reordered": _with_swapped_adjacent_lines(data, b"tier ", b"grade "),
            "truncated": data[:-1],
            "noncanonical-frame-length": _with_noncanonical_frame_length(
                data, b"operation "
            ),
            "trailing": data + b"trailing",
        }

    def test_outcome_parser_rejects_structural_mutation_table(self):
        parser = lambda value: runner_module._parse_v2_machine_record(
            value, runner_module.COOPERATIVE_OUTCOME_SCHEMA_V2
        )
        self._assert_all_rejected(
            parser, self._machine_record_mutations(self.outcome_bytes)
        )

    def test_verify_parser_rejects_structural_mutation_table(self):
        parser = lambda value: runner_module._parse_v2_machine_record(
            value, runner_module.COOPERATIVE_VERIFY_SCHEMA_V2
        )
        self._assert_all_rejected(
            parser, self._machine_record_mutations(self.verify_bytes)
        )

    def test_verify_failure_parser_rejects_structural_mutation_table(self):
        data = self.failure_bytes
        fresh_zero = _replace_line(
            data, b"reason ", b"reason 19:fresh-replay-failed"
        )
        pre_replay_nonzero = _replace_line(data, b"executions ", b"executions 1")
        nonstructural_unavailable = _replace_line(
            data, b"receipt-sha256 ", b"receipt-sha256 11:unavailable"
        )
        mutations = {
            "oversized": _oversized(data),
            "missing": _without_line(data, b"reason "),
            "duplicate": _with_duplicate_line(data, b"reason "),
            "unknown": _with_unknown_line(data),
            "reordered": _with_swapped_adjacent_lines(
                data, b"engine-sha256 ", b"manifest-id "
            ),
            "truncated": data[:-1],
            "noncanonical-frame-length": _with_noncanonical_frame_length(
                data, b"reason "
            ),
            "trailing": data + b"trailing",
            "unknown-reason": data.replace(
                b"reason 25:expected-request-mismatch\n",
                b"reason 14:invented-error\n",
                1,
            ),
            "positive-authentic": data.replace(
                b"authentic false\n", b"authentic true\n", 1
            ),
            "fresh-replay-with-zero-attempts": fresh_zero,
            "pre-replay-with-nonzero-attempts": pre_replay_nonzero,
            "nonstructural-unavailable-receipt": nonstructural_unavailable,
            "standalone-attempt-count-over-two": _replace_line(
                data, b"executions ", b"executions 3"
            ),
        }
        self._assert_all_rejected(
            runner_module._parse_v2_verification_failure, mutations
        )
        fresh_one = _replace_line(
            _replace_line(data, b"reason ", b"reason 19:fresh-replay-failed"),
            b"executions ",
            b"executions 1",
        )
        parsed = runner_module._parse_v2_verification_failure(fresh_one)
        self.assertEqual(parsed.reason, "fresh-replay-failed")
        self.assertEqual(parsed.executions, 1)

    def test_positive_looking_semantic_forgery_never_validates(self):
        for schema, canonical in (
            (runner_module.COOPERATIVE_OUTCOME_SCHEMA_V2, self.outcome_bytes),
            (runner_module.COOPERATIVE_VERIFY_SCHEMA_V2, self.verify_bytes),
        ):
            forged = canonical.replace(b"authentic true\n", b"authentic false\n", 1)
            record = runner_module._parse_v2_machine_record(forged, schema)
            self.assertEqual(record["verdict"], "CLEAN")
            self.assertFalse(record["authentic"])
            with self.assertRaises(ValueError):
                runner_module._validate_v2_machine_record(
                    record,
                    expected_schema=schema,
                    process_returncode=0,
                    manifest=self.manifest,
                    descriptor=self.descriptor,
                    features=self.features,
                    requested_revision=f"sha256:{_TARGET_REVISION}",
                )

    def test_bound_record_requires_requested_equals_verified_observation(self):
        for schema, canonical in (
            (runner_module.COOPERATIVE_OUTCOME_SCHEMA_V2, self.outcome_bytes),
            (runner_module.COOPERATIVE_VERIFY_SCHEMA_V2, self.verify_bytes),
        ):
            base = runner_module._parse_v2_machine_record(canonical, schema)
            co_mutated = dict(base)
            for field in (
                "claimed_observed_revision",
                "fresh_observed_revision",
                "verified_observed_revision",
            ):
                co_mutated[field] = "0" * 64
            with self.subTest(schema=schema, mutation="co-mutated-observations"):
                with self.assertRaisesRegex(ValueError, "requested revision"):
                    runner_module._validate_v2_machine_record(
                        co_mutated,
                        expected_schema=schema,
                        process_returncode=0,
                        manifest=self.manifest,
                        descriptor=self.descriptor,
                        features=self.features,
                        requested_revision=f"sha256:{_TARGET_REVISION}",
                    )

            unknown = dict(base)
            unknown["requested_target_revision"] = "unknown"
            with self.subTest(schema=schema, mutation="unknown-request"):
                with self.assertRaisesRegex(ValueError, "exact requested revision"):
                    runner_module._validate_v2_machine_record(
                        unknown,
                        expected_schema=schema,
                        process_returncode=0,
                        manifest=self.manifest,
                        descriptor=self.descriptor,
                        features=self.features,
                        requested_revision="unknown",
                    )


if __name__ == "__main__":
    unittest.main()
