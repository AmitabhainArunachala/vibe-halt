# VB-001 — lost-update

| field | value |
|---|---|
| `id` | VB-001 |
| `class` | lost-update |
| `source` | seeded |
| `workload` | `corpus-lost-update` |
| `expected_finding` | `oracle:no_lost_updates` |
| `recall` | found 29/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-lost-update --seed 0xD1CE --universe 1` |
| `gate` | `corpus recall gate: corpus-lost-update` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[no_lost_updates] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `acdfc32a59a4ca95dc431b848c08e5de` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `counts` | always-failures **29**; clean **71**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS` |
| `control` | fault-free/harmless universes must PASS: clean = 71 exactly (>=1) at the pinned budget; pinned clean universe 0: `vh run --workload corpus-lost-update --seed 0xD1CE --universe 0` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | the final counter and every requested-increment fact must be present and well-formed; the counter must equal the requested increments. A missing/malformed fact pair is a hard failure, never a vacuous `"" == ""` match (PR #32). |

## Mechanism

Two writers increment a shared counter via read-modify-write messages; the store applies blind last-write-wins sets with no compare-and-swap. A delayed read reply overlaps the writers' cycles: both read the same value, both write value+1, and an increment vanishes (or a stale write rolls the counter back).

## The law

The store must apply increments atomically (CAS / version check); the workload's client protocol is the bug, the store contract is the law: final counter == requested increments.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-lost-update --seed 0xD1CE --universes 100
always-failures: 29 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 71
verdict: FINDINGS   (exit 1)
```

Failing-repro receipt (universe 1): trace hash
`fd2724cd11b071f3aa26c0c5060bcc36` (61 events), fault-plan digest
`acdfc32a59a4ca95dc431b848c08e5de` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 0): trace hash
`284bbafbd9d3bf7b684ce4bd6b3b2a55` (55 events), fault-plan digest
`00264b3c560d28afe74c46eb61ec931f`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).
