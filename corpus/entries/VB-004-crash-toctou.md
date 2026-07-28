# VB-004 — crash-toctou

| field | value |
|---|---|
| `id` | VB-004 |
| `class` | crash-toctou |
| `source` | seeded |
| `workload` | `corpus-crash-toctou` |
| `expected_finding` | `oracle:act_epoch_matches_check` |
| `recall` | found **21/100** crossed-epoch manifestations at seed 0xD1CE, universe budget 100; 17 additional no-action universes are typed coverage findings, not manifestations (total finding universes 38) |
| `repro` | `vh run --workload corpus-crash-toctou --seed 0xD1CE --universe 9` |
| `gate` | `corpus recall gate: corpus-crash-toctou` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[act_epoch_matches_check] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `8970b7cdd44582e9005aa3a8ba334f93` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `divergence_check` | enabled (`divergence-check=true`); evidence: `pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)` |
| `counts` | always-failures / crossed-epoch manifestations **21**; clean **62**; divergent 0; sometimes unreached 0; invalid completions / no-opportunity coverage findings **17**; contract violations 0; total finding universes **38** (disjoint 21 + 17) |
| `expected_exit` | exit 1, `verdict: FINDINGS (see above)` |
| `control` | fault-free/harmless universes must PASS: clean = 62 exactly (>=1) at the pinned budget; pinned clean universe 1: `vh run --workload corpus-crash-toctou --seed 0xD1CE --universe 1` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | per-action check-epoch and act-epoch facts must be present and equal; a universe where no check->act action completed returns typed `InvalidAssumption` and remains a fail-closed coverage finding, but does not count as a crossed-epoch manifestation |

## Mechanism

Check-then-act across a crash window: a volatile session token is checked, the decision is remembered in application memory, and the act fires on a later timer without re-validation. A crash inside the check->act window kills the session; the act still runs on the stale check. The workload truthfully records the process epoch at check and act; the oracle demands they match per action.

## The law

Privileged actions must re-validate their guards after any restart; remembered checks do not survive a crash.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Manifestation recall pinned 2026-07-21;
classification corrected 2026-07-28.

## Contract freeze (K1 v1.2 truth correction, 2026-07-28)

All counts measured twice consecutively at engine head
`53adaea32b7a002d645f84cb62924194f29c32cb`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-crash-toctou --seed 0xD1CE --universes 100
always-failures: 21 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 17; contract violations: 0; clean: 62
verdict: FINDINGS (see above)
```

The exact split is 21 crossed-epoch manifestations, 17 typed
`InvalidAssumption` no-action coverage findings, and 62 clean universes.
The two finding axes are disjoint, so total finding universes remain 38.

Failing-repro receipt (universe 9): trace hash
`e1664370769b7189d72fb9ca05c08408` (73 events), fault-plan digest
`8970b7cdd44582e9005aa3a8ba334f93` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 1): trace hash
`ca8702bc693b1d6445808bf5b6f1909a` (56 events), fault-plan digest
`7ac9a88ebb02a59f164fa7a324ae9087`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from either the manifestation
pin or the coverage-invalid pin — in EITHER direction — is a finding to
explain and re-pin with its semantic cause, never a tolerance band. A
count that is not identical across two consecutive runs at one head
makes this entry's claim UNCHECKED and files an identity defect
(controller kill rule).

Classification changelog: the original 2026-07-21 campaign measured 21
crossed-epoch manifestations. PR #32 (`0f75659`, commit `9c8cae3`) then
made 17 no-action universes fail closed, but the 2026-07-25 K1 freeze
incorrectly added those coverage findings to manifestation recall and
reported 38/100. Core classification commit
`53adaea32b7a002d645f84cb62924194f29c32cb` moves those 17 universes to
typed `InvalidAssumption` invalid completions. The corrected split is
21 manifestation + 17 coverage-invalid + 62 clean; total finding
universes remain 38, so no fail-closed evidence was discarded.
