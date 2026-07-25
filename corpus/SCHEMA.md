# Corpus Entry Schema v1.1

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

## Contract fields (v1.1, K1 freeze 2026-07-25 — required on every entry)

These bind the full execution contract so a recall claim can be held as
an exact executable assertion (post-audit controller §6, C2-core/K1
split). Values are transcribed from CLI output at a named engine head,
never computed by hand.

| field | meaning |
|---|---|
| `root_seed` | the exact root seed every pinned count is measured at |
| `universe_budget` | the exact universe count of the pinned campaign |
| `oracle_contract` | the CLI-printed `required_oracles=[…] required_always=[…] required_sometimes=[…]` line; a missing required oracle is a contract violation, pinned 0 |
| `generator` | palette id and fault-plan schema from the CLI banner, plus the failing-repro universe's fault-plan digest |
| `schedule` | schedule policy and decision-tape requirement (`tape=` banner fact; tape digest when recorded) |
| `counts` | ALL six summary counters, exact: always-failures, clean, divergent, sometimes unreached, invalid completions, contract violations |
| `expected_exit` | the exact process exit code and verdict line of the pinned campaign |
| `control` | the fault-free control: exact clean count (must be >=1 where the model permits) plus one pinned clean universe with its expected exit |
| `required_facts` | the independent facts the oracle demands, so silence cannot become success (required-key / required-progress semantics) |

Contract laws:

- **Two-run identity.** Every pinned count is measured twice
  consecutively at the same engine head and must be identical. A count
  that differs between the two runs makes the entry UNCHECKED and
  files an identity defect — tolerance bands are never an option.
- **Drift in either direction is a finding.** A recall count moving UP
  is not automatically an improvement (it can mean a guaranteed-failure
  palette). Any deviation from a pinned count is explained and
  re-pinned with its semantic cause, in the same PR, or the entry goes
  UNCHECKED.
- **Every entry receipts its freeze.** A `## Contract freeze` section
  records the measurement commands verbatim, the engine head, the
  failing-repro receipt (trace hash, event count, fault-plan digest),
  and the clean-control receipt.

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
