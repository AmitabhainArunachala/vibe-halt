# VB-010 — resume-becomes-replay (harvested)

| field | value |
|---|---|
| `id` | VB-010 |
| `class` | retry-double-apply |
| `source` | HARVESTED: langchain-ai/langgraph issue #7361 ("When resume from a specific checkpoint_id, it becomes replay", reported 2026-03-31; regression in 1.1.x, workarounds: downgrade to 1.0.x / drop checkpoint_id / PR #7126) — https://github.com/langchain-ai/langgraph/issues/7361 |
| `workload` | `corpus-resume-replay` |
| `expected_finding` | `oracle:resume_at_most_once` |
| `recall` | found 70/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-resume-replay --seed 0xD1CE --universe 1` |
| `gate` | `corpus recall gate: corpus-resume-replay` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[resume_at_most_once] required_always=[] required_sometimes=[crash_resume]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `8cbbebd87af43b61abaf92c923d7f0f8` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `divergence_check` | enabled (`divergence-check=true`); evidence: `pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)` |
| `counts` | always-failures **70**; clean **30**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS (see above)` |
| `control` | fault-free/harmless universes must PASS: clean = 30 exactly (>=1) at the pinned budget; pinned clean universe 0: `vh run --workload corpus-resume-replay --seed 0xD1CE --universe 0` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | `applied:<step>` facts must be present and well-formed with a progress claim — "never ran at all" no longer satisfies at-most-once (PR #32); the `crash_resume` sometimes-property must be reached within the budget (`sometimes unreached` pinned 0). |

## Provenance (the real bug)

Resuming a LangGraph graph from a specific `checkpoint_id` re-executes
from the beginning instead of continuing at the interrupt point — "the
second run for resume still run from the beginning of the graph, not
interrupt trigger point." The checkpoint with the progress exists and
is readable (dropping `checkpoint_id` from the config is a workaround —
the data was fine); the resume path misuses it. Regression in 1.1.x.

## Mechanism (reduced)

A pipeline applies side-effecting steps (the outside world: tools
invoked, emails sent — crash-proof by nature) and durably fsyncs a
progress cursor after each. On crash-recovery it reads the cursor back
from durable state — and then schedules the pipeline from step 0
anyway. Every step completed before the crash lands a second time. The
palette is crashes-only (0..=2): the cursor is always on disk before a
crash can matter, so every duplicate application is the resume path
ignoring truth it demonstrably held; crash-free universes PASS.

Distinct from VB-007 (stale sweep re-dispatching one in-flight call)
and VB-002 (lossy-network retry without server dedupe): here the
duplication engine is CRASH-RECOVERY itself replaying finished work.

## The law

Recovery must resume, not replay: each step's side effect lands at most
once across all crash epochs (`applied:<step>` <= 1). The failure detail
names each replayed step and its count.

Harvested entry: counts toward the >=25 / >=80% real-recall acceptance
(corpus/SCHEMA.md law 3). Recall measured then pinned 2026-07-22.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-resume-replay --seed 0xD1CE --universes 100
always-failures: 70 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 30
verdict: FINDINGS (see above)
```

Failing-repro receipt (universe 1): trace hash
`251108dbe62d815e6c3e1d0460ba81ef` (39 events), fault-plan digest
`8cbbebd87af43b61abaf92c923d7f0f8` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 0): trace hash
`8dc7a223b2413475c5b9e774965e9750` (21 events), fault-plan digest
`b8dfda3b7737d29c93d2b74c7ae65d67`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).
