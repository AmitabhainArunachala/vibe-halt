# VB-004 — crash-toctou

| field | value |
|---|---|
| `id` | VB-004 |
| `class` | crash-toctou |
| `source` | seeded |
| `workload` | `corpus-crash-toctou` |
| `expected_finding` | `oracle:act_epoch_matches_check` |
| `recall` | found 38/100 at seed 0xD1CE, universe budget 100 (re-pinned 2026-07-25; was 21/100 — see Contract freeze changelog) |
| `repro` | `vh run --workload corpus-crash-toctou --seed 0xD1CE --universe 9` |
| `gate` | `corpus recall gate: corpus-crash-toctou` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[act_epoch_matches_check] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `8970b7cdd44582e9005aa3a8ba334f93` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `counts` | always-failures **38**; clean **62**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS` |
| `control` | fault-free/harmless universes must PASS: clean = 62 exactly (>=1) at the pinned budget; pinned clean universe 1: `vh run --workload corpus-crash-toctou --seed 0xD1CE --universe 1` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | per-action check-epoch and act-epoch facts must be present and equal, AND required-progress holds: a universe where no check->act pair was ever exercised fails closed (PR #32). |

## Mechanism

Check-then-act across a crash window: a volatile session token is checked, the decision is remembered in application memory, and the act fires on a later timer without re-validation. A crash inside the check->act window kills the session; the act still runs on the stale check. The workload truthfully records the process epoch at check and act; the oracle demands they match per action.

## The law

Privileged actions must re-validate their guards after any restart; remembered checks do not survive a crash.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-crash-toctou --seed 0xD1CE --universes 100
always-failures: 38 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 62
verdict: FINDINGS   (exit 1)
```

Failing-repro receipt (universe 9): trace hash
`e1664370769b7189d72fb9ca05c08408` (73 events), fault-plan digest
`8970b7cdd44582e9005aa3a8ba334f93` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 1): trace hash
`ca8702bc693b1d6445808bf5b6f1909a` (56 events), fault-plan digest
`7ac9a88ebb02a59f164fa7a324ae9087`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).

Changelog: 2026-07-25 re-pinned 21/100 -> 38/100 (+17).
Semantic cause: PR #32 (`0f75659`, commit `9c8cae3`) made this
oracle fail closed on required-progress — universes where the
law was never exercised previously passed in silence. Cause
verified mechanically at both sides of the merge:
`b9973f0` (pre-#32) measures 21, `0f75659` (the #32 merge)
measures 38, `ca6b37f` (current) measures 38 — same seed,
same budget, same command. The prior pin was measured before
PR #32 and never re-pinned in that PR; this entry closes that
review debt.
