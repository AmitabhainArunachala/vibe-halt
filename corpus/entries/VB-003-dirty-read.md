# VB-003 — dirty-read

| field | value |
|---|---|
| `id` | VB-003 |
| `class` | dirty-read |
| `source` | seeded |
| `workload` | `corpus-dirty-read` |
| `expected_finding` | `oracle:published_implies_durable` |
| `recall` | found **83/100** dirty-read manifestations at seed 0xD1CE, universe budget 100; 13 additional no-publication universes are typed coverage findings, not manifestations (total finding universes 96) |
| `repro` | `vh run --workload corpus-dirty-read --seed 0xD1CE --universe 0` |
| `gate` | `corpus recall gate: corpus-dirty-read` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[published_implies_durable] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `f2aa300293954b8fd2955ef1a0b666af` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `divergence_check` | enabled (`divergence-check=true`); evidence: `pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)` |
| `counts` | always-failures / dirty-read manifestations **83**; clean **4**; divergent 0; sometimes unreached 0; invalid completions / no-opportunity coverage findings **13**; contract violations 0; total finding universes **96** (disjoint 83 + 13) |
| `expected_exit` | exit 1, `verdict: FINDINGS (see above)` |
| `control` | fault-free/harmless universes must PASS: clean = 4 exactly (>=1) at the pinned budget; pinned clean universe 23: `vh run --workload corpus-dirty-read --seed 0xD1CE --universe 23` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | every `published:<record>` fact must be intact in the final durable state; a universe where no record was ever published returns typed `InvalidAssumption` and remains a fail-closed coverage finding, but does not count as a dirty-read manifestation |

## Mechanism

A reporter publishes values read from the FULL disk view — application buffer and OS cache included — as settled facts. A crash erases the volatile layers; the published values never existed durably. When publication occurred, an oracle failure names a real dirty read. A run with no publication opportunity is reported separately as an invalid-assumption coverage finding.

## The law

Downstream publication must read only committed (fsynced) state.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Manifestation recall pinned 2026-07-21;
classification corrected 2026-07-28.

## Contract freeze (K1 v1.2 truth correction, 2026-07-28)

All counts measured twice consecutively at engine head
`53adaea32b7a002d645f84cb62924194f29c32cb`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-dirty-read --seed 0xD1CE --universes 100
always-failures: 83 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 13; contract violations: 0; clean: 4
verdict: FINDINGS (see above)
```

The exact split is 83 dirty-read manifestations, 13 typed
`InvalidAssumption` no-publication coverage findings, and 4 clean
universes. The two finding axes are disjoint, so total finding universes
remain 96.

Failing-repro receipt (universe 0): trace hash
`47e73abba46b34aba1d9709023ea05e2` (47 events), fault-plan digest
`f2aa300293954b8fd2955ef1a0b666af` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 23): trace hash
`b4e38befadd09fb045fd06726b9dce50` (45 events), fault-plan digest
`207e8c1d1d65ef46e1ddb8973930cac8`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from either the manifestation
pin or the coverage-invalid pin — in EITHER direction — is a finding to
explain and re-pin with its semantic cause, never a tolerance band. A
count that is not identical across two consecutive runs at one head
makes this entry's claim UNCHECKED and files an identity defect
(controller kill rule).

Classification changelog: the original 2026-07-21 campaign measured 83
dirty-read manifestations. PR #32 (`0f75659`, commit `9c8cae3`) then made
13 no-publication universes fail closed, but the 2026-07-25 K1 freeze
incorrectly added those coverage findings to manifestation recall and
reported 96/100. Core classification commit
`53adaea32b7a002d645f84cb62924194f29c32cb` moves those 13 universes to
typed `InvalidAssumption` invalid completions. The corrected split is
83 manifestation + 13 coverage-invalid + 4 clean; total finding
universes remain 96, so no fail-closed evidence was discarded.
