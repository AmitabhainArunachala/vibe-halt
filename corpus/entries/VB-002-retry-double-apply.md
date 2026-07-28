# VB-002 — retry-double-apply

| field | value |
|---|---|
| `id` | VB-002 |
| `class` | retry-double-apply |
| `source` | seeded |
| `workload` | `corpus-retry-double-apply` |
| `expected_finding` | `oracle:exactly_once` |
| `recall` | found 76/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-retry-double-apply --seed 0xD1CE --universe 1` |
| `gate` | `corpus recall gate: corpus-retry-double-apply` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[exactly_once] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `87f4c3b995c918726e76e1977b6debc3` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `divergence_check` | enabled (`divergence-check=true`); evidence: `pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)` |
| `counts` | always-failures **76**; clean **24**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS (see above)` |
| `control` | fault-free/harmless universes must PASS: clean = 24 exactly (>=1) at the pinned budget; pinned clean universe 0: `vh run --workload corpus-retry-double-apply --seed 0xD1CE --universe 0` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | per-item applied-count facts must be present; every item must be applied exactly once — 0 (dropped) and >1 (double-apply) both fail. |

## Mechanism

The client retries un-acked appends (correct — the network is lossy) but the server applies every receipt with no idempotency key. A duplicated delivery, an append racing its own retry, or a partition-eaten ack turns one logical append into two applications. The palette's blackout budget (<=200k) sits under the retry budget (240k), so under-application is impossible: every violation is the over-apply bug.

## The law

Retries demand idempotency keys on the apply path: each requested item applied exactly once.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-retry-double-apply --seed 0xD1CE --universes 100
always-failures: 76 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 24
verdict: FINDINGS (see above)
```

Failing-repro receipt (universe 1): trace hash
`e48b00d36c8f59dd22a9b4d85305516c` (46 events), fault-plan digest
`87f4c3b995c918726e76e1977b6debc3` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 0): trace hash
`cb86db08b36a731c41a81036ac737cad` (36 events), fault-plan digest
`b99f8cdc2c8a885487345e5c516c6f71`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).
