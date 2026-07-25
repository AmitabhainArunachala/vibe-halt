# VB-003 — dirty-read

| field | value |
|---|---|
| `id` | VB-003 |
| `class` | dirty-read |
| `source` | seeded |
| `workload` | `corpus-dirty-read` |
| `expected_finding` | `oracle:published_implies_durable` |
| `recall` | found 96/100 at seed 0xD1CE, universe budget 100 (re-pinned 2026-07-25; was 83/100 — see Contract freeze changelog) |
| `repro` | `vh run --workload corpus-dirty-read --seed 0xD1CE --universe 0` |
| `gate` | `corpus recall gate: corpus-dirty-read` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[published_implies_durable] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `f2aa300293954b8fd2955ef1a0b666af` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `counts` | always-failures **96**; clean **4**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS` |
| `control` | fault-free/harmless universes must PASS: clean = 4 exactly (>=1) at the pinned budget; pinned clean universe 23: `vh run --workload corpus-dirty-read --seed 0xD1CE --universe 23` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | every `published:<record>` fact must be intact in the final durable state, AND required-progress holds: a universe where no record was ever published fails closed — silence is not success (PR #32). |

## Mechanism

A reporter publishes values read from the FULL disk view — application buffer and OS cache included — as settled facts. A crash erases the volatile layers; the published values never existed durably. Crash-free universes pass (final shutdown persists everything), so the finding always names a real dirty read.

## The law

Downstream publication must read only committed (fsynced) state.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-dirty-read --seed 0xD1CE --universes 100
always-failures: 96 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 4
verdict: FINDINGS   (exit 1)
```

Failing-repro receipt (universe 0): trace hash
`47e73abba46b34aba1d9709023ea05e2` (47 events), fault-plan digest
`f2aa300293954b8fd2955ef1a0b666af` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 23): trace hash
`b4e38befadd09fb045fd06726b9dce50` (45 events), fault-plan digest
`207e8c1d1d65ef46e1ddb8973930cac8`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).

Changelog: 2026-07-25 re-pinned 83/100 -> 96/100 (+13).
Semantic cause: PR #32 (`0f75659`, commit `9c8cae3`) made this
oracle fail closed on required-progress — universes where the
law was never exercised previously passed in silence. Cause
verified mechanically at both sides of the merge:
`b9973f0` (pre-#32) measures 83, `0f75659` (the #32 merge)
measures 96, `ca6b37f` (current) measures 96 — same seed,
same budget, same command. The prior pin was measured before
PR #32 and never re-pinned in that PR; this entry closes that
review debt.
