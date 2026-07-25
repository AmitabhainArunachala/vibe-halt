"""Adversarial tests for the C3-honesty slice (controller section 7).

Pre-repair (evidence.py:11,19 at dfc0551): EvidenceReport was publicly
constructible with manufactured defaults — reproducibility_score=1.0 and,
when no summary was supplied, a minted "All properties held across N
universes" — without any runner-owned evidence. These tests pin the
fail-closed contract that replaces it: no publicly constructible evidence
object can carry a perfection claim, because the Python execution path is
quarantined and supplies no runner evidence.

Run from clients/python:  python3 -m unittest tests.test_evidence -v
"""

import unittest

from vibe_halt import MultiverseRunner
from vibe_halt.core.evidence import EvidenceReport, ManufacturedEvidenceError


class ManufacturedDefaultsAreDead(unittest.TestCase):
    def test_bare_construction_no_longer_mints_a_perfect_report(self):
        # Pre-repair this constructed reproducibility_score=1.0 and the
        # summary "All properties held across 5 universes" from nothing.
        with self.assertRaises(TypeError):
            EvidenceReport(universes_run=5)

    def test_perfection_score_is_unconstructible(self):
        with self.assertRaises(ManufacturedEvidenceError):
            EvidenceReport(
                universes_run=5,
                violations=[],
                reproducibility_score=1.0,
                summary="ran without incident",
            )

    def test_all_properties_held_summary_is_unconstructible(self):
        for summary in (
            "All properties held across 5 universes",
            "all properties held",
            "ALL PROPERTIES HELD (trust me)",
        ):
            with self.subTest(summary=summary):
                with self.assertRaises(ManufacturedEvidenceError):
                    EvidenceReport(
                        universes_run=5,
                        violations=[],
                        reproducibility_score=0.5,
                        summary=summary,
                    )

    def test_empty_summary_is_rejected_not_manufactured(self):
        # Pre-repair an empty summary was silently replaced with a claim.
        with self.assertRaises(ValueError):
            EvidenceReport(
                universes_run=5,
                violations=[],
                reproducibility_score=0.5,
                summary="",
            )

    def test_score_outside_unit_interval_is_rejected(self):
        for score in (-0.1, 1.5, float("nan"), float("inf")):
            with self.subTest(score=score):
                with self.assertRaises(ValueError):
                    EvidenceReport(
                        universes_run=5,
                        violations=[],
                        reproducibility_score=score,
                        summary="questionable",
                    )

    def test_honest_report_still_constructs_and_serializes(self):
        report = EvidenceReport(
            universes_run=5,
            violations=[{"universe": 3, "property": "durability"}],
            reproducibility_score=0.4,
            summary="1 violation across 5 universes",
        )
        d = report.to_dict()
        self.assertEqual(d["universes_run"], 5)
        self.assertEqual(d["reproducibility_score"], 0.4)
        self.assertEqual(len(d["violations"]), 1)
        self.assertNotIn("all properties held", d["summary"].lower())

    def test_execution_quarantine_stays_closed(self):
        # PR #1 hardening-loop-4 BLOCKER 3: the runner must not simulate.
        with self.assertRaises(NotImplementedError):
            MultiverseRunner("some-target")


if __name__ == "__main__":
    unittest.main()
