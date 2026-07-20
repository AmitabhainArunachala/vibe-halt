# VB-004 — crash-toctou

| field | value |
|---|---|
| `id` | VB-004 |
| `class` | crash-toctou |
| `source` | seeded |
| `workload` | `corpus-crash-toctou` |
| `expected_finding` | `oracle:act_epoch_matches_check` |
| `recall` | found 21/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-crash-toctou --seed 0xD1CE --universe 9` |
| `gate` | `corpus recall gate: corpus-crash-toctou` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |

## Mechanism

Check-then-act across a crash window: a volatile session token is checked, the decision is remembered in application memory, and the act fires on a later timer without re-validation. A crash inside the check->act window kills the session; the act still runs on the stale check. The workload truthfully records the process epoch at check and act; the oracle demands they match per action.

## The law

Privileged actions must re-validate their guards after any restart; remembered checks do not survive a crash.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.
