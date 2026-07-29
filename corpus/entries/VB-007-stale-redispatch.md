# VB-007 — stale-sweep re-dispatch (harvested)

| field | value |
|---|---|
| `id` | VB-007 |
| `class` | retry-double-apply |
| `source` | HARVESTED: langchain-ai/langgraph issue #7417 ("Long tool calls (~180s+) silently re-executed from checkpoint on LangGraph Cloud", reported 2026-04-05, reproducible on langgraph 1.1.3–1.1.6 / Cloud Plus) — https://github.com/langchain-ai/langgraph/issues/7417 |
| `workload` | `corpus-stale-redispatch` |
| `expected_finding` | `oracle:exactly_once_dispatch` |
| `recall` | found 91/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-stale-redispatch --seed 0xD1CE --universe 0` |
| `gate` | `corpus recall gate: corpus-stale-redispatch` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[exactly_once_dispatch] required_always=[] required_sometimes=[redispatch_fired]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `02d424ef2a581d30d8652a4a668605f7` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `divergence_check` | enabled (`divergence-check=true`); evidence: `pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)` |
| `counts` | always-failures **91**; clean **9**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS (see above)` |
| `control` | fault-free/harmless universes must PASS: clean = 9 exactly (>=1) at the pinned budget; pinned clean universe 11: `vh run --workload corpus-stale-redispatch --seed 0xD1CE --universe 11` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | per-task applied-count facts must show exactly one apply; the `redispatch_fired` sometimes-property must be reached within the budget (`sometimes unreached` pinned 0), so a palette that never opens the redispatch window cannot pass silently. |

## Provenance (the real bug)

LangGraph Cloud's stale-run detection (`BG_JOB_HEARTBEAT` hardcoded at
120s; queue sweep every 240s) marks a tool call that runs longer than
~180s as stale and re-dispatches it from the last checkpoint **while
the original execution is still running**; `CancelledError` sits in
`ALL_RETRIABLE_EXCEPTIONS`, so the swept run restarts from pending.
Both instances complete successfully — duplicate side effects, 2–3×
redundant cost, duplicate tool invocations with identical arguments in
traces. Reported with reproduction details in the issue above; the
mechanism (a liveness sweep with a fixed deadline re-enqueuing
merely-slow in-flight work, with no idempotency key at the effect site)
is the harvested shape.

## Mechanism (reduced)

A dispatcher sends tasks to a worker and arms a stale-sweep deadline
per task. If the completion has not returned by the deadline, the task
is presumed dead and re-sent — but the palette is DELAY-ONLY, so the
original dispatch (or its completion) is merely slow, never lost. The
worker applies every receipt with no idempotency key. The delayed
original and the sweep's re-dispatch both apply: one logical task, two
applications.

Distinct from VB-002 (seeded retry double-apply): there the retry
answers *real* loss under a lossy palette; here **nothing is ever
lost** — every duplicate is the sweep wrongly presuming a slow call
dead, which is exactly the harvested defect's shape.

## The law

Each dispatched task's effect must land at most once regardless of how
slow its delivery or completion is (idempotency key / in-flight check
at the worker; or the sweep must verify liveness rather than assume a
deadline). Final `applied:<task>` == 1 for every task; the failure
detail names each over-applied task and its count.

Harvested regression entry: its real-issue provenance and pinned
manifestation count test the reduced mechanism, but it receives no
retrospective holdout credit (`corpus/SCHEMA.md` laws 3–4). Recall measured
then pinned 2026-07-22.
VB-006 is intentionally skipped: reserved for the convergence C2
same-timestamp race (docs/prompts/CONVERGENCE_CAMPAIGN_EXECUTOR_2026-07-22.md §4).

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-stale-redispatch --seed 0xD1CE --universes 100
always-failures: 91 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 9
verdict: FINDINGS (see above)
```

Failing-repro receipt (universe 0): trace hash
`326b0dd419e897ce07142ca83adab1ed` (41 events), fault-plan digest
`02d424ef2a581d30d8652a4a668605f7` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 11): trace hash
`a273e996565012f8e439de388b54226a` (33 events), fault-plan digest
`5edda3474de62145a2ab9dec1635042a`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).
