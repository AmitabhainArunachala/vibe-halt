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

Harvested entry: counts toward the >=25 / >=80% real-recall acceptance
(corpus/SCHEMA.md law 3). Recall measured then pinned 2026-07-22.
VB-006 is intentionally skipped: reserved for the convergence C2
same-timestamp race (docs/prompts/CONVERGENCE_CAMPAIGN_EXECUTOR_2026-07-22.md §4).
