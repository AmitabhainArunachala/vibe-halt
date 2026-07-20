# Corpus Entry Schema v1

One file per bug under `corpus/entries/`, named `VB-<nnn>-<slug>.md`.
An entry without a pinned, mechanically-checked recall gate is NOT a
corpus entry — it is an anecdote (track
`vibe-bug-corpus-2026-07` non-goal).

## Required fields

| field | meaning |
|---|---|
| `id` | `VB-<nnn>` — stable, never reused |
| `class` | bug class slug (`lost-update`, `retry-double-apply`, `dirty-read`, `crash-toctou`, `fsync-lie-hole`, …) |
| `source` | where the bug came from: `seeded` (written for the corpus) or a citation to the real code/PR it was harvested from |
| `workload` | the `vh` workload name that embodies it |
| `expected_finding` | the exact anchored finding line class the rig must produce (e.g. `oracle:exactly_once`) |
| `recall` | the pinned recall claim: seed, universe budget, and the observed find count at pin time (`found F/N at seed S`) |
| `repro` | one command reproducing a single failing universe deterministically |
| `gate` | the `scripts/gate.sh` gate name holding the recall claim green |
| `tier` | determinism tier of the recall evidence (Tier 1 for engine-owned workloads) |

## Laws

1. **Recall is measured, then pinned.** The `recall` field records what a
   real campaign found at the pinned seed — never a hoped-for number.
   The gate then holds exactly that claim.
2. **Every entry names its tier** (DETERMINISM_TIERS.md: "deterministic"
   without a tier is an uncited claim).
3. **Seeded entries are lower-bound evidence only.** They prove the rig
   CAN find the class; they say nothing about real-code recall
   (build-plan risk 4: demo-overfitting). Harvested entries are the
   metric that counts toward the >=25 / >=80% acceptance.
